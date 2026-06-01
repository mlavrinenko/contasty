package calc

import (
	"fmt"
	"strings"
)

// Running total seed.
const Seed = 0

// Calculator sums integers.
type Calculator struct {
	total int
}

func (c *Calculator) Add(lhs, rhs int) int {
	return lhs + rhs
}

func (c *Calculator) Describe() string {
	banner := "Calculator"
	return banner
}

func Helper(name string) string {
	return fmt.Sprintf("Hello, %s, this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output here for the go fixture.", name)
}

func TestAdd(t *testing.T) {
	c := &Calculator{}
	if c.Add(1, 2) != 3 {
		t.Fatal("bad")
	}
}
