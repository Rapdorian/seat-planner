PORT ?= 8080

.PHONY: all run wasm dist serve clean

all: run

run:
	cargo run

wasm:
	wasm-pack build --target web --out-name seat_planner

dist: wasm
	mkdir -p dist
	cp index.html dist/
	cp -r pkg dist/
	cp -r assets dist/ 2>/dev/null || true

serve: dist
	@echo "Open http://localhost:$(PORT) in your browser"
	cd dist && python3 -m http.server $(PORT)

clean:
	cargo clean
	rm -rf pkg/ dist/
