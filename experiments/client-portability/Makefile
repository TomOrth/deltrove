GO ?= go
VERSION := 0.1.0-spike
LDFLAGS := -s -w -X main.version=$(VERSION)
TARGETS := linux/amd64 linux/arm64 darwin/amd64 darwin/arm64

.PHONY: test vet build-matrix reproducibility

test:
	$(GO) test -race ./...

vet:
	$(GO) vet ./...

build-matrix:
	@set -eu; mkdir -p dist; for target in $(TARGETS); do \
		os=$${target%/*}; arch=$${target#*/}; \
		echo "building $$os/$$arch"; \
		GOOS=$$os GOARCH=$$arch CGO_ENABLED=0 $(GO) build -trimpath -buildvcs=false -ldflags '$(LDFLAGS)' -o "dist/deltrove-$$os-$$arch" ./cmd/deltrove; \
	done

reproducibility:
	@set -eu; mkdir -p dist; for target in $(TARGETS); do \
		os=$${target%/*}; arch=$${target#*/}; \
		GOOS=$$os GOARCH=$$arch CGO_ENABLED=0 $(GO) build -trimpath -buildvcs=false -ldflags '$(LDFLAGS)' -o dist/first ./cmd/deltrove; \
		GOOS=$$os GOARCH=$$arch CGO_ENABLED=0 $(GO) build -trimpath -buildvcs=false -ldflags '$(LDFLAGS)' -o dist/second ./cmd/deltrove; \
		cmp dist/first dist/second; rm dist/first dist/second; \
		echo "reproducible $$os/$$arch"; \
	done
