#!/usr/bin/env bash
# LOC-Analyse: Produktivcode vs. Tests pro Rust-Datei
set -euo pipefail

cd "$(dirname "$0")/../src"

printf "%-38s %6s %6s %6s %6s\n" "File" "Prod" "Tests" "Total" "Ratio"
printf "%-38s %6s %6s %6s %6s\n" "----" "----" "-----" "-----" "-----"

sum_prod=0
sum_tests=0

while IFS= read -r f; do
    total=$(wc -l < "$f")
    test_start=$(grep -n "^#\[cfg(test)\]" "$f" | head -1 | cut -d: -f1 || true)

    if [ -n "$test_start" ]; then
        prod=$((test_start - 1))
        tests=$((total - test_start + 1))
    else
        prod=$total
        tests=0
    fi

    sum_prod=$((sum_prod + prod))
    sum_tests=$((sum_tests + tests))

    if [ "$prod" -gt 0 ] && [ "$tests" -gt 0 ]; then
        ratio=$(awk "BEGIN {printf \"%.1f:1\", $tests/$prod}")
    else
        ratio="-"
    fi

    printf "%-38s %6d %6d %6d %6s\n" "$f" "$prod" "$tests" "$total" "$ratio"
done < <(find . -name "*.rs" | sort)

printf "%-38s %6s %6s %6s %6s\n" "----" "----" "-----" "-----" "-----"

if [ "$sum_prod" -gt 0 ]; then
    total_ratio=$(awk "BEGIN {printf \"%.1f:1\", $sum_tests/$sum_prod}")
else
    total_ratio="-"
fi

printf "%-38s %6d %6d %6d %6s\n" "TOTAL" "$sum_prod" "$sum_tests" "$((sum_prod + sum_tests))" "$total_ratio"
