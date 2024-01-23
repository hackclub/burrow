tun_num := $(shell ifconfig | awk -F 'utun|[: ]' '/utun[0-9]/ {print $$2}' | tail -n 1)
cargo_console := RUST_BACKTRACE=1 RUST_LOG=debug RUSTFLAGS='--cfg tokio_unstable' cargo run --all-features
cargo_norm := RUST_BACKTRACE=1 RUST_LOG=debug cargo run

check:
	@cargo check

build:
	@cargo run build

daemon-console:
	@$(cargo_console) daemon

daemon:
	@$(cargo_norm) daemon

start:
	@$(cargo_norm) start

test-dns:
	@sudo route delete 8.8.8.8
	@sudo route add 8.8.8.8 -interface utun$(tun_num)
	@dig @8.8.8.8 hackclub.com

test-https:
	@sudo route delete 193.183.0.162
	@sudo route add 193.183.0.162 -interface utun$(tun_num)
	@curl -vv https://search.marginalia.nu

test-http:
	@sudo route delete 146.190.62.39
	@sudo route add 146.190.62.39 -interface utun$(tun_num)
	@curl -vv 146.190.62.39:80
