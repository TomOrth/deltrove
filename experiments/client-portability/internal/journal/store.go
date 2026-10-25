// Package journal contains the small SQLite journal used by the portability toy.
package journal

import (
	"context"
	"database/sql"
	"fmt"
	"net/url"

	_ "modernc.org/sqlite"
)

// Store is the persistence boundary for this local journal experiment.
type Store interface {
	Write(context.Context, string, string) error
	Read(context.Context, string) (string, error)
	Close() error
}

type sqliteStore struct{ db *sql.DB }

// Open creates or opens a local SQLite journal and ensures its schema exists.
func Open(ctx context.Context, path string) (Store, error) {
	db, err := openSQLite(ctx, path)
	if err != nil {
		return nil, err
	}
	if _, err := db.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS journal (key TEXT PRIMARY KEY, value TEXT NOT NULL)`); err != nil {
		if closeErr := db.Close(); closeErr != nil {
			return nil, fmt.Errorf("initialize SQLite journal: %w (close: %v)", err, closeErr)
		}
		return nil, fmt.Errorf("initialize SQLite journal: %w", err)
	}
	return &sqliteStore{db: db}, nil
}

// openSQLite applies the profile used by the CLI and subprocess recovery test.
func openSQLite(ctx context.Context, path string) (*sql.DB, error) {
	query := url.Values{}
	query.Add("_pragma", "journal_mode(DELETE)")
	query.Add("_pragma", "synchronous(FULL)")
	dsn := (&url.URL{Scheme: "file", Path: path, RawQuery: query.Encode()}).String()
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, fmt.Errorf("open SQLite journal: %w", err)
	}
	db.SetMaxOpenConns(1)
	if err := db.PingContext(ctx); err != nil {
		if closeErr := db.Close(); closeErr != nil {
			return nil, fmt.Errorf("initialize SQLite connection: %w (close: %v)", err, closeErr)
		}
		return nil, fmt.Errorf("initialize SQLite connection: %w", err)
	}
	mode, synchronous, err := readSQLiteProfile(ctx, db)
	if err != nil {
		if closeErr := db.Close(); closeErr != nil {
			return nil, fmt.Errorf("verify SQLite connection profile: %w (close: %v)", err, closeErr)
		}
		return nil, fmt.Errorf("verify SQLite connection profile: %w", err)
	}
	if mode != "delete" || synchronous != 2 {
		if closeErr := db.Close(); closeErr != nil {
			return nil, fmt.Errorf("unexpected SQLite profile journal_mode=%q synchronous=%d (close: %v)", mode, synchronous, closeErr)
		}
		return nil, fmt.Errorf("unexpected SQLite profile journal_mode=%q synchronous=%d", mode, synchronous)
	}
	return db, nil
}

func readSQLiteProfile(ctx context.Context, db *sql.DB) (string, int, error) {
	var journalMode string
	if err := db.QueryRowContext(ctx, `PRAGMA journal_mode`).Scan(&journalMode); err != nil {
		return "", 0, fmt.Errorf("read SQLite journal mode: %w", err)
	}
	var synchronous int
	if err := db.QueryRowContext(ctx, `PRAGMA synchronous`).Scan(&synchronous); err != nil {
		return "", 0, fmt.Errorf("read SQLite synchronous mode: %w", err)
	}
	return journalMode, synchronous, nil
}

// Write atomically replaces one journal value in a SQLite transaction.
func (s *sqliteStore) Write(ctx context.Context, key, value string) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("begin journal transaction: %w", err)
	}
	if _, err := tx.ExecContext(ctx, `INSERT INTO journal(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value`, key, value); err != nil {
		if rollbackErr := tx.Rollback(); rollbackErr != nil {
			return fmt.Errorf("write journal: %w (rollback: %v)", err, rollbackErr)
		}
		return fmt.Errorf("write journal: %w", err)
	}
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("commit journal transaction: %w", err)
	}
	return nil
}

// Read returns one journal value, reporting sql.ErrNoRows when absent.
func (s *sqliteStore) Read(ctx context.Context, key string) (string, error) {
	var value string
	if err := s.db.QueryRowContext(ctx, `SELECT value FROM journal WHERE key = ?`, key).Scan(&value); err != nil {
		return "", fmt.Errorf("read journal key %q: %w", key, err)
	}
	return value, nil
}

// Close releases the SQLite connection.
func (s *sqliteStore) Close() error {
	if err := s.db.Close(); err != nil {
		return fmt.Errorf("close SQLite journal: %w", err)
	}
	return nil
}
