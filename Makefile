.PHONY: fmt test vet build package clean install

fmt:
	cd structure-of-knowledge && cargo fmt --all

test:
	cd structure-of-knowledge && cargo test

vet:
	cd structure-of-knowledge && cargo clippy --all-targets -- -D warnings

build:
	mkdir -p structure-of-knowledge/bin
	cd structure-of-knowledge && cargo build --release --bin sok
	cp structure-of-knowledge/target/release/sok structure-of-knowledge/bin/sok

package:
	mkdir -p dist
	tar --exclude='.DS_Store' \
		--exclude='structure-of-knowledge/bin' \
		--exclude='structure-of-knowledge/target' \
		-czf dist/structure-of-knowledge-skill.tar.gz \
		LICENSE README.md structure-of-knowledge

clean:
	rm -rf structure-of-knowledge/bin structure-of-knowledge/target dist

install: build
	install_root="$${CODEX_HOME:-$${HOME}/.codex}/skills"; \
	install_dir="$$install_root/structure-of-knowledge"; \
	mkdir -p "$$install_root"; \
	rm -rf "$$install_dir"; \
	mkdir -p "$$install_dir"; \
	tar -C structure-of-knowledge \
		--exclude='./bin' \
		--exclude='./target' \
		-cf - . | tar -C "$$install_dir" -xf -; \
	mkdir -p "$$install_dir/bin"; \
	cp structure-of-knowledge/bin/sok "$$install_dir/bin/sok"; \
	echo "Installed structure-of-knowledge skill to $$install_dir"
