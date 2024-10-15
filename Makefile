.PHONY: up
up:
	@docker-compose up

.PHONY: down
down:
	@docker-compose down

.PHONY: tests
tests:
	@APP_JWT_SECRET='deadbeef' \
		MOCK_ADMIN_URL='http://127.0.0.1:11111/__admin__/register-snap' \
		HOST='0.0.0.0' \
		PORT='8080' \
		cargo test

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
