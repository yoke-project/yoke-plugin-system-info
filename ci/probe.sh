# What the watch asks of this Plugin, and the case it answers.
PLUGIN=com.yoke.system-info
UNIT=system-info
CASE=yoke-plugin-system-info:the-watch.01

# std: yoke-plugin-system-info:the-watch.01
probe() { # <instance>
  yoke plugin grant "$PLUGIN" query.info.answer --instance "$1" > /dev/null || { echo "the grant was refused"; return 1; }
  yoke unit restart "$UNIT" --instance "$1" > /dev/null || { echo "the unit did not restart"; return 1; }
  sleep 3
  local said; said="$(yoke unit ask "$UNIT" system.info --instance "$1" 2>&1)" || { echo "the question was refused: $said"; return 1; }
  grep -q '"hostname"' <<<"$said" || { echo "the answer holds no hostname: $said"; return 1; }
}
