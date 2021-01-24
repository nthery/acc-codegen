#!/bin/bash

# Compile compiler (if needed), run it on specified input, assemble compiler
# output and link it against runtime.
driver() {
	cargo run "$1" > output.s
	if (( $? != 0 )); then
		echo "compiler error"
		exit 1
	fi
	nasm -fmacho64 output.s
	if (( $? != 0 )); then
		echo "assembler error"
		cat -n output.s
		exit 1
	fi
	clang -g runtime.c output.o
	if (( $? != 0 )); then
		echo "linker error"
		exit 1
	fi
	./a.out
}

# Run driver on given input and check it matches expected output.
test() {
	local -r input="$1"
	local -r expected="$2"

	echo "TEST: $input"
	local -r got=$(driver "$input")
	if [[ "$got" != "$expected" ]]; then
		echo "FAILURE: $input: expected $expected, got $got"
		exit 1
	fi
}

#
# All test cases
#

test 7 			7
test 72+ 		9
test 12+3+ 		6
test 123*+ 		7
test 12+3* 		9
test 12345++++  	15
test 12*34*+		14
test "1;2"		2
test a			0
test "a2=;a1+"		3
test "ba2==;b1+"	3
