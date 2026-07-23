.PHONY: fmt test vet build package clean install

CLI_MANIFEST := cli/Cargo.toml
CLI_BINARY := cli/target/release/sok
SKILL_SOURCE := structure-of-knowledge

fmt:
	cargo fmt --manifest-path $(CLI_MANIFEST) --all

test:
	cargo test --manifest-path $(CLI_MANIFEST) --locked

vet:
	cargo clippy --manifest-path $(CLI_MANIFEST) --locked --all-targets -- -D warnings

build:
	cargo build --manifest-path $(CLI_MANIFEST) --locked --release --bin sok

package:
	mkdir -p dist
	tar --exclude='.DS_Store' \
		-czf dist/structure-of-knowledge-skill.tar.gz \
		LICENSE README.md $(SKILL_SOURCE)

clean:
	rm -rf cli/target dist

install: build
	set -eu; \
	install_root="$${CODEX_HOME:-$${HOME}/.codex}/skills"; \
	install_dir="$$install_root/structure-of-knowledge"; \
	mkdir -p "$$install_root"; \
	staging_dir="$$(mktemp -d "$$install_root/.structure-of-knowledge.XXXXXX")"; \
	trap 'rm -rf "$$staging_dir"' EXIT; \
	tar -C $(SKILL_SOURCE) --exclude='./.DS_Store' -cf - . | tar -C "$$staging_dir" -xf -; \
	test -f "$$staging_dir/SKILL.md"; \
	mkdir -p "$$staging_dir/bin"; \
	cp $(CLI_BINARY) "$$staging_dir/bin/sok"; \
	chmod 0755 "$$staging_dir/bin/sok"; \
	test -x "$$staging_dir/bin/sok"; \
	version_output="$$("$$staging_dir/bin/sok" --version)"; \
	case "$$version_output" in *"(rust)") ;; *) echo "Refusing to install a non-Rust SoK CLI: $$version_output" >&2; exit 1 ;; esac; \
	rm -rf "$$install_dir"; \
	mv "$$staging_dir" "$$install_dir"; \
	trap - EXIT; \
	echo "Installed structure-of-knowledge skill to $$install_dir"
