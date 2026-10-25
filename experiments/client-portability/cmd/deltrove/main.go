package main

import (
	"context"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/TomOrth/deltrove/clients/go/generated"
	"github.com/spf13/cobra"
)

var version = "dev"

func main() {
	if err := newRootCommand().ExecuteContext(context.Background()); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}
}

func newRootCommand() *cobra.Command {
	root := &cobra.Command{Use: "deltrove", Short: "Deltrove client portability toy", SilenceUsage: true, SilenceErrors: true}
	root.AddCommand(newAuthCommand(), newQueryCommand(), newJournalCommand(), &cobra.Command{
		Use:   "version",
		Short: "Print the portability toy version",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			_, err := fmt.Fprintln(cmd.OutOrStdout(), version)
			return err
		},
	})
	return root
}

func newAuthCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "auth",
		Short: "Show authentication support",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			_, err := fmt.Fprintln(cmd.OutOrStdout(), "Authentication is unsupported in this portability toy; session=mock-local. No credentials are accepted or stored.")
			return err
		},
	}
}

func newQueryCommand() *cobra.Command {
	query := &cobra.Command{Use: "query", Short: "Query the provisional HTTP API", Args: cobra.NoArgs}
	query.AddCommand(&cobra.Command{
		Use:   "health",
		Short: "Read server liveness with the generated Go client",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			baseURL := os.Getenv("DELTROVE_BASE_URL")
			if baseURL == "" {
				baseURL = "http://127.0.0.1:8080"
			}
			client, err := deltroveclient.NewClientWithResponses(baseURL)
			if err != nil {
				return fmt.Errorf("create generated API client: %w", err)
			}
			ctx, cancel := context.WithTimeout(cmd.Context(), 3*time.Second)
			defer cancel()
			response, err := client.GetHealthLiveWithResponse(ctx)
			if err != nil {
				return fmt.Errorf("query generated API client: %w", err)
			}
			if response.JSON200 == nil || response.JSON200.Status != deltroveclient.Alive {
				return fmt.Errorf("unexpected liveness response: %#v", response.JSON200)
			}
			_, err = fmt.Fprintln(cmd.OutOrStdout(), response.JSON200.Status)
			return err
		},
	})
	return query
}

func newJournalCommand() *cobra.Command {
	var dbPath string
	command := &cobra.Command{Use: "journal-demo", Short: "Write a local SQLite journal toy", Args: cobra.NoArgs}
	command.Flags().StringVar(&dbPath, "db", "deltrove-local.db", "SQLite database path")
	command.RunE = func(cmd *cobra.Command, _ []string) (runErr error) {
		ctx, cancel := context.WithTimeout(cmd.Context(), 3*time.Second)
		defer cancel()
		store, err := openJournalStore(ctx, dbPath)
		if err != nil {
			return err
		}
		defer func() {
			if closeErr := store.Close(); closeErr != nil {
				runErr = errors.Join(runErr, closeErr)
			}
		}()
		if err := store.Write(ctx, "demo", "local journal entry"); err != nil {
			return err
		}
		_, err = fmt.Fprintln(cmd.OutOrStdout(), "journal entry committed")
		return err
	}
	return command
}
