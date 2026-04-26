TEST_CACHE := ./test/llc/cache/
TEST_LANG  := ./test/llc/lang/
LBC_TEST   := ./test/LimbusCompany_Data/Lang/

.PHONY: default
default:
	@make help

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  make run     - Clean and run the project"
	@echo "  make clean   - Remove test directories"

.PHONY: run
run: clean
	cargo run

.PHONY: clean
clean:
	rm -rf $(TEST_CACHE)
	rm -rf $(TEST_LANG)
	rm -rf $(LBC_TEST)
