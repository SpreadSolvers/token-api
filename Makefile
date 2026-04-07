.PHONY: build
build:
	docker build -t token-api .

.PHONY: run
run:
	docker run --rm -d --name token-api -p 7777:8080 -e RUST_LOG=debug -e APP_ENV=development -e DATABASE_URL=file:$(pwd)/data/token-api.db -v "$(PWD)/db:/data" token-api

.PHONY: stop
stop:
	docker stop token-api

.PHONY: logs
logs:
	docker logs -f token-api

.PHONY: clean