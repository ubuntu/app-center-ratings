db-up:
	docker compose up --detach
	sleep 3

db-up-test:
	dotenvy -f .env_files/test.env docker compose up --detach
	sleep 3

db-down:
	docker compose down

test:
	dotenvy -f .env_files/test-server.env cargo test --package ratings_new

full-test: db-up-test test

clean:
	docker rmi ratings-postgres -f
	cargo clean
