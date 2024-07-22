tun := $(shell ifconfig -l | sed 's/ /\n/g' | grep utun | tail -n 1)
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

stop:
	@$(cargo_norm) stop

status:
	@$(cargo_norm) server-status

tunnel-config:
	@$(cargo_norm) tunnel-config

test-dns:
	@sudo route delete 8.8.8.8
	@sudo route add 8.8.8.8 -interface $(tun)
	@dig @8.8.8.8 hackclub.com

test-https:
	@sudo route delete 193.183.0.162
	@sudo route add 193.183.0.162 -interface $(tun)
	@curl -vv https://search.marginalia.nu

v4_target := 146.190.62.39
test-http:
	@sudo route delete ${v4_target}
	@sudo route add ${v4_target} -interface $(tun)
	@curl -vv ${v4_target}:80

test-ipv4:
	@sudo route delete ${v4_target}
	@sudo route add ${v4_target} -interface $(tun)
	@ping ${v4_target}

v6_target := 2001:4860:4860::8888
test-ipv6:
	@sudo route delete ${v6_target}
	@sudo route -n add -inet6 ${v6_target} -interface $(tun)
	@echo preparing
	@sudo ping6 -v ${v6_target}
