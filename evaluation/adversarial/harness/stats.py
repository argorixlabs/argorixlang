"""Interval estimation for the campaign.

Proportions are always reported as n/N with a Wilson 95% interval.  When the
numerator is zero the rule-of-three upper bound is reported instead of a
"0%" claim.
"""

from __future__ import annotations

import math
from typing import Any

Z95 = 1.959963984540054


def wilson(successes: int, total: int, z: float = Z95) -> tuple[float | None, float | None]:
    if total <= 0:
        return (None, None)
    proportion = successes / total
    denominator = 1.0 + (z * z) / total
    centre = proportion + (z * z) / (2 * total)
    spread = z * math.sqrt(
        (proportion * (1.0 - proportion) + (z * z) / (4 * total)) / total
    )
    lower = (centre - spread) / denominator
    upper = (centre + spread) / denominator
    return (max(0.0, round(lower, 6)), min(1.0, round(upper, 6)))


def rule_of_three(total: int) -> float | None:
    """Upper 95% bound on an unobserved event after `total` trials."""
    if total <= 0:
        return None
    return round(3.0 / total, 6)


def proportion(successes: int, total: int, *, label: str = "") -> dict[str, Any]:
    lower, upper = wilson(successes, total)
    result: dict[str, Any] = {
        "label": label,
        "numerator": successes,
        "denominator": total,
        "point": round(successes / total, 6) if total else None,
        "wilson95": {"lower": lower, "upper": upper},
        "text": f"{successes}/{total}",
    }
    if total and successes == 0:
        result["rule_of_three_upper"] = rule_of_three(total)
        result["note"] = (
            "zero observed events; reported as an upper bound, never as 0% risk"
        )
    return result


def quantiles(values: list[float]) -> dict[str, float | None]:
    if not values:
        return {"n": 0, "median": None, "q1": None, "q3": None, "iqr": None, "p95": None}
    ordered = sorted(values)

    def percentile(fraction: float) -> float:
        if len(ordered) == 1:
            return round(ordered[0], 3)
        position = fraction * (len(ordered) - 1)
        low = math.floor(position)
        high = math.ceil(position)
        if low == high:
            return round(ordered[int(position)], 3)
        weight = position - low
        return round(ordered[low] * (1 - weight) + ordered[high] * weight, 3)

    q1 = percentile(0.25)
    q3 = percentile(0.75)
    return {
        "n": len(ordered),
        "median": percentile(0.5),
        "q1": q1,
        "q3": q3,
        "iqr": round(q3 - q1, 3),
        "p95": percentile(0.95),
        "min": round(ordered[0], 3),
        "max": round(ordered[-1], 3),
    }


__all__ = ["proportion", "quantiles", "rule_of_three", "wilson"]
