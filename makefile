.PHONY: fmt lint install install-bar install-ctl install-all uninstall check-install

ARGS ?=

fmt:
	dprint fmt

lint:
	cargo clippy \
		--workspace \
		--all-targets \
		--all-features \
		--fix \
		--allow-dirty \
		--allow-staged \
		-- -D warnings

	ruff check installer --fix

install:
	./scripts/install.sh install $(ARGS)

install-all:
	$(MAKE) install ARGS="--bar --ctl --clip --notify $(ARGS)"

uninstall:
	./scripts/install.sh uninstall $(ARGS)

check-install:
	./scripts/install.sh check $(ARGS)

