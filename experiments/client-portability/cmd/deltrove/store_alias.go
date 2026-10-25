package main

import (
	"context"

	"github.com/TomOrth/deltrove/experiments/client-portability/internal/journal"
)

func openJournalStore(ctx context.Context, path string) (journal.Store, error) {
	return journal.Open(ctx, path)
}
