module 0x42::test {
	struct S0(u8);

	struct S1(bool, S0);

	enum E1 {
		V1(S0),
		V2(S1)
	}

	fun simple_arity_mismatch1(x: S0) {
		let S0() = x;
	}

	fun simple_arity_mismatch2(x: S0) {
		let S0(_x, _y) = x;
	}

	fun nested_arity_mismatch(x: S1) {
		let S1(_x, S0(_y, _z)) = x;
	}

	fun match_invalid(x: E1) {
		match (x) {
			E1::V1(S0(_)) => {},
			E1::V1(S0(_x, _y)) => {},
			E1::V2(S1(_x, S0(_y, _z))) => {}
		}
	}
}
