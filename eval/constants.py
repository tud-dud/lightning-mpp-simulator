from matplotlib.cm import get_cmap

linestyles_dict = dict(
    [
        ("solid", (0, ())),
        # ("loosely dotted", (0, (1, 10))),
        # ("dotted", (0, (1, 5))),
        ("dashdot", "dashdot"),
        ("densely dotted", (0, (1, 1))),
        ("loosely dashed", (0, (5, 10))),
        ("dashed", (0, (4, 4))),
        ("densely dashed", (0, (5, 1))),
        ("loosely dotted", (0, (1, 5, 1, 5))),
        ("dashdotted", (0, (3, 5, 1, 5))),
        ("densely dashdotted", (0, (3, 1, 1, 1))),
        (
            "densely dotdashed",
            (
                0,
                (
                    1,
                    1,
                    3,
                    3,
                ),
            ),
        ),
        ("dashdotdotted", (0, (3, 5, 1, 5, 1, 5))),
        ("densely dashdotdotted", (0, (3, 1, 1, 1, 1, 1))),
    ]
)

LINESTYLE_MinFeeSingle = linestyles_dict["densely dashed"]
LINESTYLE_MinFeeMulti = linestyles_dict["loosely dashed"]
LINESTYLE_MaxProbSingle = linestyles_dict["densely dotted"]
LINESTYLE_MaxProbMulti = linestyles_dict["loosely dotted"]

"""
COLOUR_MAP_NAME = "Paired"
COLOUR_MAP = get_cmap(COLOUR_MAP_NAME).colors
COLOUR_MinFeeSingle = COLOUR_MAP[1]
COLOUR_MinFeeMulti = COLOUR_MAP[0]
COLOUR_MaxProbSingle = COLOUR_MAP[5]
COLOUR_MaxProbMulti = COLOUR_MAP[4]
"""
COLOUR_MinFeeSingle = "#2166AC"
COLOUR_MinFeeMulti = "#4393C3"
COLOUR_MaxProbSingle = "#B2182B"
COLOUR_MaxProbMulti = "#D6604D"

MARKER_MinFeeSingle = "o"
MARKER_MinFeeMulti = "d"
MARKER_MaxProbSingle = "x"
MARKER_MaxProbMulti = "+"

X_TICKS_LABELS = [
    f"{100:,}",
    f"{500:,}",
    f"{1000:,}",
    f"{5000:,}",
    f"{10000:,}",
    f"{50000:,}",
    f"{100000:,}",
    f"{500000:,}",
    f"{1000000:,}",
    f"{5000000:,}",
    f"{10000000:,}",
]

IEEE_FIG_WIDTH = 3.3
IEEE_FIG_HEIGHT = 2.5
