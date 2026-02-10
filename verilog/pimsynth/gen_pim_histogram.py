import os

import matplotlib.pyplot as plt
import numpy as np


def count_asm_statements(filepath):
    if not os.path.exists(filepath):
        return 0
    count = 0
    with open(filepath, "r") as f:
        for line in f:
            if "asm" in line:
                count += 1
    # Subtract 2 as requested by the user
    return max(0, count - 2)


def main():
    benchmarks = []
    pimsynth_counts = []
    eqmap_counts = []

    # Find all _pimsynth.c files to identify benchmarks
    files = os.listdir(".")
    pimsynth_files = [f for f in files if f.endswith("_pimsynth.c")]

    for ps_file in sorted(pimsynth_files):
        benchmark_base = ps_file.replace("_pimsynth.c", "")
        eqmap_file = benchmark_base + "_eqmap.c"

        if os.path.exists(eqmap_file):
            ps_count = count_asm_statements(ps_file)
            eq_count = count_asm_statements(eqmap_file)

            benchmarks.append(benchmark_base)
            pimsynth_counts.append(ps_count)
            eqmap_counts.append(eq_count)

    if not benchmarks:
        print("No benchmark pairs found.")
        return

    # Plotting
    x = np.arange(len(benchmarks))
    width = 0.35

    fig, ax = plt.subplots(figsize=(12, 6))
    ax.bar(x - width / 2, pimsynth_counts, width, label="PIMsynth")
    ax.bar(x + width / 2, eqmap_counts, width, label="EqMap")

    ax.set_ylabel("# of PIM ops")
    ax.set_xlabel("Benchmarks")
    plt.title("Bar Chart Title", fontsize=12, y=1.055, color="#444", ha="center")

    ax.set_title(
        "Analog PIM Ops Comparison: PIMsynth vs Eqmap",
        fontsize=12,
        y=1.2,
        ha="center",
    )
    ax.text(
        0.5,
        1.02,
        "tool: pim\n flags: --rules analog-pim.celllang --no-assert --min-depth -s 100000 -t 10 -n 10000000\n rules filehash: 8730dc3fe6d4ab26, num_regs: 6",
        ha="center",
        fontsize=9,
        color="#666",
        transform=ax.transAxes,
    )
    ax.set_xticks(x)
    ax.set_xticklabels(benchmarks, rotation=45, ha="right")
    ax.legend()

    fig.tight_layout()
    plt.savefig("pim_histogram.png")
    print("Histogram saved as pim_histogram.png")

    plt.savefig("pim_histogram.png")
    print("Histogram saved as pim_histogram.png")

    print("Histogram saved as pim_histogram.png")


if __name__ == "__main__":
    main()
