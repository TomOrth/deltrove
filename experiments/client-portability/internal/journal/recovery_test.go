package journal

import (
	"bufio"
	"context"
	"database/sql"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	_ "modernc.org/sqlite"
)

func TestInterruptedTransactionRecovery(t *testing.T) {
	for _, test := range []struct {
		name       string
		mode       string
		wantValue  string
		wantNew    string
		wantAbsent bool
	}{
		{name: "unacknowledged write rolls back to preimage", mode: "before-commit", wantValue: "before", wantAbsent: true},
		{name: "acknowledged commit survives process death", mode: "after-commit", wantValue: "after", wantNew: "partial"},
		{name: "new row from interrupted write is absent", mode: "before-commit", wantAbsent: true},
	} {
		t.Run(test.name, func(t *testing.T) {
			dbPath := filepath.Join(t.TempDir(), "journal.db")
			seedJournal(t, dbPath)
			marker := runCrashWorker(t, dbPath, test.mode)
			if test.wantAbsent {
				if got := readJournal(t, dbPath, "new"); got != "" {
					t.Fatalf("interrupted insert survived: got %q", got)
				}
			}
			if test.wantValue != "" {
				if got := readJournal(t, dbPath, "existing"); got != test.wantValue {
					t.Fatalf("after marker %q, existing value = %q, want %q", marker, got, test.wantValue)
				}
			}
			if test.wantNew != "" {
				if got := readJournal(t, dbPath, "new"); got != test.wantNew {
					t.Fatalf("after marker %q, new value = %q, want %q", marker, got, test.wantNew)
				}
			}
		})
	}
}

func TestSQLiteProfile(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	db, err := openSQLite(ctx, filepath.Join(t.TempDir(), "profile.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := db.Close(); err != nil {
			t.Errorf("close profile database: %v", err)
		}
	}()
	mode, synchronous, err := readSQLiteProfile(ctx, db)
	if err != nil {
		t.Fatal(err)
	}
	if mode != "delete" || synchronous != 2 {
		t.Fatalf("SQLite profile = journal_mode:%q synchronous:%d; want delete/full(2)", mode, synchronous)
	}
}

func TestCrashWorker(t *testing.T) {
	mode := os.Getenv("DELTROVE_CRASH_WORKER_MODE")
	if mode == "" {
		return
	}
	if len(os.Args) < 3 {
		os.Exit(90)
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Minute)
	defer cancel()
	db, err := openSQLite(ctx, os.Args[len(os.Args)-1])
	if err != nil {
		os.Exit(91)
	}
	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		os.Exit(92)
	}
	if _, err := tx.ExecContext(ctx, `INSERT INTO journal(key, value) VALUES('existing', 'after') ON CONFLICT(key) DO UPDATE SET value=excluded.value`); err != nil {
		os.Exit(93)
	}
	if _, err := tx.ExecContext(ctx, `INSERT INTO journal(key, value) VALUES('new', 'partial')`); err != nil {
		os.Exit(94)
	}
	switch mode {
	case "before-commit":
		if _, err := fmt.Fprintln(os.Stdout, "before-commit"); err != nil {
			os.Exit(97)
		}
	case "after-commit":
		if err := tx.Commit(); err != nil {
			os.Exit(95)
		}
		if _, err := fmt.Fprintln(os.Stdout, "committed"); err != nil {
			os.Exit(98)
		}
	default:
		os.Exit(96)
	}
	if _, err := io.Copy(io.Discard, os.Stdin); err != nil {
		os.Exit(99)
	}
	_ = db.Close()
	os.Exit(0)
}

func seedJournal(t *testing.T, path string) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	store, err := Open(ctx, path)
	if err != nil {
		t.Fatal(err)
	}
	if err := store.Write(ctx, "existing", "before"); err != nil {
		t.Fatal(err)
	}
	if err := store.Close(); err != nil {
		t.Fatal(err)
	}
}

func runCrashWorker(t *testing.T, dbPath, mode string) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 8*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, os.Args[0], "-test.run=^TestCrashWorker$", "--", dbPath)
	cmd.Env = append(os.Environ(), "DELTROVE_CRASH_WORKER_MODE="+mode)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatal(err)
	}
	stdin, err := cmd.StdinPipe()
	if err != nil {
		t.Fatal(err)
	}
	if err := cmd.Start(); err != nil {
		t.Fatalf("start helper process: %v", err)
	}
	line, err := bufio.NewReader(stdout).ReadString('\n')
	if err != nil {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatalf("read causal helper marker: %v", err)
	}
	wantMarker := "before-commit"
	if mode == "after-commit" {
		wantMarker = "committed"
	}
	if strings.TrimSpace(line) != wantMarker {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatalf("helper marker = %q, want %q", strings.TrimSpace(line), wantMarker)
	}
	if err := cmd.Process.Kill(); err != nil {
		_ = stdin.Close()
		_ = cmd.Wait()
		t.Fatalf("kill owned helper process after marker: %v", err)
	}
	if err := stdin.Close(); err != nil {
		t.Errorf("close helper stdin after kill: %v", err)
	}
	if err := cmd.Wait(); err == nil {
		t.Fatal("helper process unexpectedly exited cleanly after kill")
	}
	return strings.TrimSpace(line)
}

func readJournal(t *testing.T, path, key string) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	store, err := Open(ctx, path)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := store.Close(); err != nil {
			t.Errorf("close reopened store: %v", err)
		}
	}()
	value, err := store.Read(ctx, key)
	if key == "new" && errors.Is(err, sql.ErrNoRows) {
		return ""
	}
	if err != nil {
		t.Fatal(fmt.Errorf("read reopened store: %w", err))
	}
	return value
}
