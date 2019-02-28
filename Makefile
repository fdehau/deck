
CHROMIUM_PATH := $(shell which chromium)

%.html: %.md
	cargo run build < $< > $@

%.pdf: export PUPPETEER_EXECUTABLE_PATH=$(CHROMIUM_PATH)
%.pdf: %.html
	cd tools && node pdf.js $(shell pwd)/$< $(shell pwd)/$@
