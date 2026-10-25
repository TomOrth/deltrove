package main

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/TomOrth/deltrove/clients/go/generated"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run() error {
	baseURL := os.Getenv("DELTROVE_BASE_URL")
	if baseURL == "" {
		baseURL = "http://127.0.0.1:8080"
	}
	client, err := deltroveclient.NewClientWithResponses(baseURL)
	if err != nil {
		return fmt.Errorf("create generated API client: %w", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	response, err := client.GetHealthLiveWithResponse(ctx)
	if err != nil {
		return fmt.Errorf("call generated API client: %w", err)
	}
	if response.JSON200 == nil || response.JSON200.Status != "alive" {
		return fmt.Errorf("unexpected liveness response: %#v", response.JSON200)
	}
	missing, err := client.GetOperationWithResponse(ctx, "smoke-missing")
	if err != nil {
		return fmt.Errorf("call generated operation lookup: %w", err)
	}
	problem := missing.ApplicationproblemJSON404
	if missing.StatusCode() != 404 || problem == nil || problem.Code != "operation_not_found" || problem.Status != 404 || problem.Retryable {
		return fmt.Errorf("unexpected operation lookup problem: status=%d body=%#v", missing.StatusCode(), problem)
	}
	fmt.Println("generated Go client received alive and typed operation_not_found")
	return nil
}
