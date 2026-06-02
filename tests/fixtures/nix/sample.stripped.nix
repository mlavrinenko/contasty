{ pkgs }:
let
  add = a: b: a + b;
  banner = "[…CTY]";
in {
  inherit add banner;
}
