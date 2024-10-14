.PHONY: up
up:
	@docker-compose up

.PHONY: down
down:
	@docker-compose down

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
