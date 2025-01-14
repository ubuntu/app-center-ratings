.PHONY: build
build:
	@cargo build --release

.PHONY: docker-build
docker-build:
	@docker build -f docker/prod/Dockerfile .

.PHONY: up
up:
	@docker-compose up

.PHONY: up-detached
up-detached:
	@docker-compose up --detach

.PHONY: down
down:
	@docker-compose down

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
integration-test: clear-db-data
	@APP_JWT_SECRET='deadbeef' \
		MOCK_ADMIN_URL='http://127.0.0.1:11111/__admin__/register-snap' \
		HOST='0.0.0.0' \
		PORT='8080' \
		cargo test --test '*' $(ARGS)

.PHONY: test-all
test-all: db-test integration-test

.PHONY: wait-for-server
wait-for-server:
	@echo "Waiting for ratings server to start on port 8080..."
	@until docker-compose ps | grep -E '^ratings\s' | grep healthy; do \
		echo "..."; \
		sleep 1; \
	done

.PHONY: ci-test
ci-test: up-detached wait-for-server test-all down

.PHONY: clear-db-data
clear-db-data:
	@docker-compose exec -T db psql -U postgres ratings < tests/clear-db.sql

.PHONY: rebuild-local
rebuild-local:
	@docker-compose build

.PHONY: rm-db
rm-db:
	@docker-compose rm db
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
