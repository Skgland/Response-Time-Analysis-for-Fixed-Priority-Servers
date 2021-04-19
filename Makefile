latex/recreate.pdf: latex/recreate.tex latex/data/*.csv
	mkdir -p latex/build/figures/ latex/figures/
	cd latex && pdflatex \
		-synctex=1 \
		-interaction=nonstopmode \
		-output-directory=build \
		-shell-escape \
		recreate.tex

latex/data/*.csv: run_gen

target/debug/rta-for-fps-latex-gen: rta-for-fps-latex-gen/src/**
	cargo build

run_gen: target/debug/rta-for-fps-latex-gen
	mkdir -p latex/data/
	cargo run