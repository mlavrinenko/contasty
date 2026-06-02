{ pkgs }:
# Calculator expression.
let
  add = a: b: a + b;
  banner = "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the nix fixture and here is some extra padding text appended to comfortably exceed the limit";
in {
  inherit add banner;
}
