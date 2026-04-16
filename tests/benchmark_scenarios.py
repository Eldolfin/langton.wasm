"""Shared scenario definitions for benchmarks.

Each entry maps a scenario key to its display label and simulation parameters.
Add, remove, or rename scenarios here — all other benchmark scripts pick them up
automatically.
"""

# Ordered dict: insertion order defines table/summary row order.
SCENARIOS: dict[str, dict] = {
    "light": {
        "label": "Simple (1 ants)",
        "number_of_ants": 1,
        "cell_size": 20,
        "final_speed": 1000000,
    },
    "medium": {
        "label": "Medium (50 ants)",
        "number_of_ants": 50,
        "cell_size": 10,
        "final_speed": 10000,
    },
    "heavy": {
        "label": "Heavy (500 ants)",
        "number_of_ants": 500,
        "cell_size": 5,
        "final_speed": 1000,
    },
    "ultra_heavy": {
        "label": "Ultra Heavy (100000 ants)",
        "number_of_ants": 100000,
        "cell_size": 5,
        "final_speed": 10,
    },
    "full_retention": {
        "label": "Full retention (50 ants, α=255)",
        "number_of_ants": 50,
        "cell_size": 10,
        "alpha_retention": 255,
        "final_speed": 100000,
    },
}
