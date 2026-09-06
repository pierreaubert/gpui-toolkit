# Unreleased

## Fixed

- Documented the Metal custom-draw bounds contract (dispatch passes the
  logical pixels the draw was painted with; draws own the
  logical-to-physical mapping) and locked it with the
  `custom_draw_bounds_arrive_logical` regression test.
