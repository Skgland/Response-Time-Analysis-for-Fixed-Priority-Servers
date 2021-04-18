latex/recreate.pdf: latex/recreate.tex latex/data/*.csv
	cd latex && pdflatex \
		-synctex=1 \
		-interaction=nonstopmode \
		-output-directory=build \
		-shell-escape \
		recreate.tex

latex/data/*.csv: run_gen


.PHONY: build_gen
build_gen:
	cargo build

run_gen: build_gen
	cargo run