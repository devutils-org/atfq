#!/bin/sh
# Runs in each guest, as an unprivileged user: every build staged under
# dist/distroproof/ goes through the same checks, and cell.json records them.
# Exit status: 1 when the static build fails a check; the published release is
# measured, not judged.
set -u
here=$PWD
out=$here/cell.json
log=$(mktemp)
static_failed=0

json_str() { printf '"%s"' "$(printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n\t' '  ')"; }

mcp_session() {
    printf '%s\n' \
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"distroproof","version":"0"}}}' \
        '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
        '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
        '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"run_probe","arguments":{"name":"mem.summary"}}}' \
        | timeout 20 "$1" mcp
}

printf '{"target":%s,"builds":{' "$(json_str "${DP_TARGET:-unknown}")" > "$out"
sep=
for dir in "$here"/dist/distroproof/*/; do
    build=$(basename "$dir")
    bin=${dir}atfq
    printf '%s%s:{' "$sep" "$(json_str "$build")" >> "$out"
    sep=,
    if ! "$bin" --version > "$log" 2>&1; then
        echo "[$build] does not start: $(head -n 1 "$log")"
        printf '"starts":false,"error":%s}' "$(json_str "$(head -n 3 "$log")")" >> "$out"
        [ "$build" != static ] || static_failed=1
        continue
    fi
    echo "[$build] $(cat "$log")"
    failed=
    check() {
        name=$1; shift
        if "$@" > "$log" 2>&1; then echo "[$build] ok    $name"
        else echo "[$build] FAIL  $name: $(head -n 2 "$log")"; failed="$failed $name"; fi
    }
    check toc "$bin" toc
    for hub in memory disk cpu network boot; do check "$hub" "$bin" "$hub" --json; done
    check card-why "$bin" free --why
    check search "$bin" search zombie
    check fuzzy sh -c "'$bin' memroy 2>&1 | grep -q memory"
    check ask-redact sh -c "out=\$('$bin' ask memory --redact 'why is RAM full?') && ! printf '%s' \"\$out\" | grep -q '$HOME'"
    mcp_session "$bin" > "$log" 2>&1
    if [ "$(grep -c '"result"' "$log")" -ge 3 ]; then echo "[$build] ok    mcp"
    else echo "[$build] FAIL  mcp: $(head -c 200 "$log")"; failed="$failed mcp"; fi
    # Every probe: exit 0 whatever the machine lacks; the status says what it found.
    statuses=
    for probe in $("$bin" probe 2>/dev/null | sed -n 's/^ *atfq probe \([a-z0-9._-]*\).*/\1/p'); do
        if "$bin" probe "$probe" --json > "$log" 2>&1; then
            status=$(sed -n 's/^  "status": "\([a-z_]*\)".*/\1/p' "$log" | head -n 1)
            statuses="$statuses${statuses:+,}$(json_str "$probe"):$(json_str "${status:-unknown}")"
        else
            echo "[$build] FAIL  probe $probe: $(head -n 2 "$log")"
            failed="$failed probe:$probe"
            statuses="$statuses${statuses:+,}$(json_str "$probe"):\"crashed\""
        fi
    done
    fails=
    for f in $failed; do fails="$fails${fails:+,}$(json_str "$f")"; done
    printf '"starts":true,"failed":[%s],"probes":{%s}}' "$fails" "$statuses" >> "$out"
    if [ -n "$failed" ] && [ "$build" = static ]; then static_failed=1; fi
done
printf '}}\n' >> "$out"
rm -f "$log"
exit "$static_failed"
