test_cache := "./test/llc/cache/"
test_lang := "./test/llc/lang/"
lbc_test := "./test/LimbusCompany_Data/Lang/"


default:
    just --list

run: clean
	cargo run

clean:
	rm -rf {{test_cache}}
	rm -rf {{test_lang}}
	rm -rf {{lbc_test}}


