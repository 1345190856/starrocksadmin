.PHONY: help build docker-build docker-up docker-down clean

# Project paths
PROJECT_ROOT := $(shell pwd)
BACKEND_DIR := $(PROJECT_ROOT)/backend
FRONTEND_DIR := $(PROJECT_ROOT)/frontend
BUILD_DIR := $(PROJECT_ROOT)/build
DIST_DIR := $(BUILD_DIR)/dist

# Default target - show help
help:
	@echo "StarRocks Admin - Build Commands:"
	@echo ""
	@echo "Build:"
	@echo "  make build              - Build backend and frontend, then create distribution package"
	@echo "  make docker-build       - Build Docker image"
	@echo "  make docker-up          - Start Docker container (uses existing image)"
	@echo "  make docker-down        - Stop Docker container"
	@echo "  make clean              - Clean build artifacts"
	@echo "  make watch              - Watch for changes and build automatically"
	@echo ""

# Watch and build
watch:
	@echo "Watching for changes..."
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x "make build"; \
	else \
		echo "cargo-watch not found. Please install it with: cargo install cargo-watch"; \
		echo "Falling back to simple loop (requires fswatch)..."; \
		fswatch -o . | xargs -n1 -I{} make build; \
	fi

# Build both backend and frontend, then create distribution package
build:
	@echo "Building StarRocks Admin..."
	@echo "Step 1: Building frontend (required for embedding)..."
	@bash build/build-frontend.sh
	@echo ""
	@echo "Step 2: Running clippy checks on backend..."
	@if [ "$(SKIP_CLIPPY)" = "true" ]; then \
		echo "Skipping clippy checks as requested (SKIP_CLIPPY=true)"; \
	else \
		cd $(BACKEND_DIR) && cargo clippy --release --all-targets -- --deny warnings --allow clippy::uninlined-format-args; \
		echo "✓ Clippy checks passed!"; \
	fi
	@echo ""
	@echo "Step 3: Building backend (with embedded frontend)..."
	@bash build/build-backend.sh
	@echo "Build complete! Output: $(DIST_DIR)"
	@echo "Creating distribution package..."
	@TIMESTAMP=$$(date +"%Y%m%d"); \
	PACKAGE_NAME="starrocks-admin-$$TIMESTAMP.tar.gz"; \
	PACKAGE_PATH="$(DIST_DIR)/$$PACKAGE_NAME"; \
	echo "Package name: $$PACKAGE_NAME"; \
	TEMP_DIR=$$(mktemp -d); \
	mkdir -p $$TEMP_DIR/starrocks-admin; \
	cp -r bin conf lib data logs migrations $$TEMP_DIR/starrocks-admin/ 2>/dev/null || true; \
	cd $$TEMP_DIR && tar -czf "$$PACKAGE_PATH" starrocks-admin; \
	rm -rf $$TEMP_DIR; \
	echo "Package created: $$PACKAGE_PATH"; \
	echo "To extract: tar -xzf $$PACKAGE_NAME"

# Build Docker image
docker-build:
	@echo "Building Docker image..."
	@docker build -f deploy/docker/Dockerfile -t starrocks-admin:latest .

# Start Docker container without rebuild (use existing image)
docker-up:
	@echo "Starting Docker container (using existing image)..."
	@cd deploy/docker && docker compose up -d

# Stop Docker container
docker-down:
	@echo "Stopping Docker container..."
	@cd deploy/docker && docker compose down

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(BUILD_DIR)
	@cd $(BACKEND_DIR) && cargo clean
	@cd $(FRONTEND_DIR) && rm -rf dist node_modules/.cache
	@echo "Clean complete!"