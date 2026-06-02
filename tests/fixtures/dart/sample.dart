import 'dart:math';
import 'package:test/test.dart';

/// Calculator sums integers.
class Calculator {
  int total = 0;

  int add(int lhs, int rhs) {
    return lhs + rhs;
  }

  String greet() => "Calculator";

  String banner() {
    return "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the dart fixture and here is some extra padding text";
  }
}

void main() {
  test('adds', () {
    expect(Calculator().add(1, 2), 3);
  });
}
