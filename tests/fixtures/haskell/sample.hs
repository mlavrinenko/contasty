module Calculator (add, banner) where

import Data.List (foldl')
import Data.Maybe

-- Sum two integers.
add :: Int -> Int -> Int
add a b = a + b

banner :: String
banner = "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the haskell fixture and here is some extra padding text appended to comfortably exceed the limit"
