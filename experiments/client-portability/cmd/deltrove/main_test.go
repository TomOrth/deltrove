package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestCommands(t *testing.T) {
	tests := []struct {
		name      string
		args      []string
		want      string
		wantEmpty bool
		wantError bool
	}{
		{name: "auth clearly unsupported", args: []string{"auth"}, want: "Authentication is unsupported"},
		{name: "auth carries only mock label", args: []string{"auth"}, want: "session=mock-local"},
		{name: "auth rejects positional credentials", args: []string{"auth", "secret"}, wantEmpty: true, wantError: true},
		{name: "version is deterministic", args: []string{"version"}, want: "dev"},
		{name: "query requires service", args: []string{"query", "health"}, wantError: true},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if test.wantError {
				t.Setenv("DELTROVE_BASE_URL", "http://127.0.0.1:1")
			}
			command := newRootCommand()
			var output strings.Builder
			command.SetOut(&output)
			command.SetErr(&output)
			command.SetArgs(test.args)
			err := command.ExecuteContext(context.Background())
			if (err != nil) != test.wantError {
				t.Fatalf("ExecuteContext() error = %v, wantError %v", err, test.wantError)
			}
			if test.want != "" && !strings.Contains(output.String(), test.want) {
				t.Fatalf("output %q does not contain %q", output.String(), test.want)
			}
			if test.wantEmpty && strings.TrimSpace(output.String()) != "" {
				t.Fatalf("rejected command unexpectedly produced output %q", output.String())
			}
		})
	}
}

func TestGeneratedClientQueryUsesHTTP(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/health/live" {
			t.Errorf("request = %s %s, want GET /health/live", r.Method, r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"status":"alive"}`))
	}))
	defer server.Close()
	t.Setenv("DELTROVE_BASE_URL", server.URL)
	command := newRootCommand()
	var output strings.Builder
	command.SetOut(&output)
	command.SetArgs([]string{"query", "health"})
	if err := command.ExecuteContext(context.Background()); err != nil {
		t.Fatalf("generated HTTP query failed: %v", err)
	}
	if got := strings.TrimSpace(output.String()); got != "alive" {
		t.Fatalf("query output = %q, want alive", got)
	}
}
