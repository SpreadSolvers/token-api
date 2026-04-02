.PHONY: build
build:
	docker build -t token-api .

.PHONY: run
run:
	docker run -d --name token-api -p 7777:8080 -v "$(PWD)/db:/data" token-api

.PHONY: stop
stop:
	docker stop token-api

.PHONY: clean