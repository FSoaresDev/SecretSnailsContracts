.PHONY: start-server
start-server: # CTRL+C to stop
	docker run -it --rm \
		-p 26657:26657 -p 26656:26656 -p 1337:1337 \
		-v $$(pwd):/root/code \
		-v $$(pwd)/secretdevConfig.toml:/root/.secretd/config/config.toml \
		--name secretdev enigmampc/secret-network-sw-dev:latest

.PHONY: server-debug-print-output
server-debug-print-output:
	docker logs -f secretdev --tail 1000 | grep "debug_print: "

.PHONY: compile
compile:
	RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
	mkdir -p build
	cp ./target/wasm32-unknown-unknown/release/secret_snails_nft.wasm ./build/secret_snails_nft.wasm
	cat ./build/secret_snails_nft.wasm | gzip -9 > ./build/secret_snails_nft.wasm.gz
	cp ./scripts/localdev/snip20_reference_impl.wasm ./build/snip20_reference_impl.wasm
	cat ./build/snip20_reference_impl.wasm | gzip -9 > ./build/snip20_reference_impl.wasm.gz
	cp ./target/wasm32-unknown-unknown/release/secret_snails_minter.wasm ./build/secret_snails_minter.wasm
	cat ./build/secret_snails_minter.wasm | gzip -9 > ./build/secret_snails_minter.wasm.gz
	
clean:
	cargo clean
	rm -f ./build/*
	rm -rf contracts/*/target
	rm -rf ./target/*

.PHONY: setup-devchain
setup-devchain:
	make compile
	bash scripts/localdev/localdev_setup.sh

.PHONY: deploy-testnet
deploy-testnet:
	bash scripts/testnet/deploy.sh

.PHONY: tests
tests: 
	cd tests && npm run test