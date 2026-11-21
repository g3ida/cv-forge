from cv_builder.paginate import Unit, paginate_column


def stack(section: str, heights: list[float], start: float = 0.0, title: float = 10.0) -> list[Unit]:
    """Units of one section laid out back to back, the first one including the title."""
    units, y = [], start
    for i, h in enumerate(heights):
        top = y
        bottom = y + (title if i == 0 else 0) + h
        units.append(Unit(section, i, top, bottom))
        y = bottom
    return units


def test_everything_fits_on_one_page():
    units = stack("work", [30, 30, 30])
    pages, overflows = paginate_column("left", units, {"work": 10}, 200, 300)
    assert [len(p) for p in pages] == [3]
    assert not overflows


def test_items_flow_to_next_page_with_repeated_title():
    units = stack("work", [40, 40, 40, 40])  # bottoms: 50, 90, 130, 170
    pages, _ = paginate_column("left", units, {"work": 10}, 100, 100)
    assert [[u.index for u in p] for p in pages] == [[0, 1], [2, 3]]
    # page 2 must leave room for the repeated title: 10 + 80 = 90 <= 100
    assert not pages[1][0].opens_section


def test_repeated_title_height_is_accounted_for():
    units = stack("work", [40, 45, 45])  # bottoms: 50, 95, 140
    pages, _ = paginate_column("left", units, {"work": 10}, 50, 100)
    # page 2 holds items 1 and 2: 140 - 50 + 10 (title) = 100 -> fits exactly
    assert [[u.index for u in p] for p in pages] == [[0], [1, 2]]


def test_first_item_skips_short_first_page():
    units = stack("work", [80])
    pages, overflows = paginate_column("left", units, {"work": 10}, 50, 100)
    assert [len(p) for p in pages] == [0, 1]
    assert not overflows


def test_item_taller_than_a_page_is_reported():
    units = stack("work", [500, 20])
    pages, overflows = paginate_column("left", units, {"work": 10}, 100, 100)
    assert len(overflows) == 1 and overflows[0].unit.index == 0
    assert [[u.index for u in p] for p in pages] == [[0], [1]]


def test_sections_share_pages():
    units = stack("summary", [20]) + stack("skills", [20, 20], start=35)
    pages, _ = paginate_column("right", units, {"summary": 10, "skills": 10}, 1000, 1000)
    assert [(u.section, u.index) for u in pages[0]] == [("summary", 0), ("skills", 0), ("skills", 1)]


def test_tolerance_decides_whether_a_hair_too_much_still_fits():
    units = stack("work", [40, 50.3])  # bottoms: 50, 100.3
    # The default slack keeps both on one page.
    pages, _ = paginate_column("left", units, {"work": 10}, 100, 100)
    assert [[u.index for u in p] for p in pages] == [[0, 1]]
    # A strict fit moves the second item on, so the 0.3px cannot spill onto an extra page.
    pages, _ = paginate_column("main", units, {"work": 0}, 100, 100, tolerance=0)
    assert [[u.index for u in p] for p in pages] == [[0], [1]]
    exact = stack("work", [40, 50])
    assert [len(p) for p in paginate_column("main", exact, {"work": 0}, 100, 100, tolerance=0)[0]] == [2]
    assert [len(p) for p in paginate_column("main", exact, {"work": 0}, 100, 100, tolerance=-0.1)[0]] == [1, 1]
