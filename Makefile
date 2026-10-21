COMPOSE ?= docker compose

.PHONY: build
build:
	@cargo build --release

.PHONY: docker-build
docker-build:
	@docker build -f docker/prod/Dockerfile .

.PHONY: up
up:
	@$(COMPOSE) up

.PHONY: up-detached
up-detached:
	@$(COMPOSE) up --detach

.PHONY: down
down:
	@$(COMPOSE) down

.PHONY: test
test:
	@cargo test --lib

.PHONY: db-test
db-test:
	@APP_HOST='0.0.0.0' \
		APP_PORT='8080' \
		APP_JWT_SECRET='deadbeef' \
		APP_SNAPCRAFT_IO_URI='localhost:11111/' \
		APP_POSTGRES_URI='postgresql://migration_user:strongpassword@localhost:5432/ratings' \
		cargo test --lib --features db_tests

.PHONY: integration-test
integration-test: restart-server clear-db-data
	@APP_JWT_SECRET='deadbeef' \
		MOCK_ADMIN_URL='http://127.0.0.1:11111/__admin__/register-snap' \
		HOST='0.0.0.0' \
		PORT='8080' \
		cargo test --test '*' $(ARGS)

.PHONY: test-all
test-all: db-test integration-test

.PHONY: wait-for-server
# Health goes "unhealthy" while a slow build outlasts the healthcheck retries,
# then "healthy" once it serves, so only a timeout means the server failed.
WAIT_FOR_SERVER_TIMEOUT ?= 900
wait-for-server:
	@echo "Waiting for ratings server to start on port 8080..."
	@elapsed=0; \
	until [ "$$($(COMPOSE) ps ratings --format '{{.Health}}' 2>/dev/null)" = healthy ]; do \
		if [ $$elapsed -ge $(WAIT_FOR_SERVER_TIMEOUT) ]; then \
			echo "ratings server not healthy after $(WAIT_FOR_SERVER_TIMEOUT)s, recent logs:"; \
			$(COMPOSE) logs --tail 20 ratings 2>/dev/null; \
			exit 1; \
		fi; \
		sleep 5; elapsed=$$((elapsed + 5)); \
	done
	@echo "ratings server is healthy"

.PHONY: restart-server
# The ratings container rebuilds on file changes in the background; restarting
# it forces a build of the current source and waits until that build is serving.
restart-server:
	@$(COMPOSE) restart ratings
	@$(MAKE) --no-print-directory wait-for-server

.PHONY: ci-test
ci-test: up-detached wait-for-server test-all down

.PHONY: clear-db-data
clear-db-data:
	@$(COMPOSE) exec -T db psql -U postgres ratings < tests/clear-db.sql

.PHONY: rebuild-local
rebuild-local:
	@$(COMPOSE) build

.PHONY: rm-db
rm-db:
	@$(COMPOSE) rm db
	@docker volume rm -f postgres_data

.PHONY: rm-volumes
rm-volumes:
	@docker volume rm -f target-cache
	@docker volume rm -f cargo-cache

.PHONY: db-shell
db-shell:
	@docker exec -it ratings-db psql -U postgres ratings

.PHONY: ratings-shell
ratings-shell:
	@docker exec -it ratings bash
