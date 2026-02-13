#!/usr/bin/env bash
set -euo pipefail

umask 077

log() {
  printf '[grail-browser-service] %s\n' "$*" >&2
}

require_bin() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log "missing required binary: $1"
    exit 1
  fi
}

is_valid_port() {
  local p="$1"
  [[ "$p" =~ ^[0-9]+$ ]] && ((p >= 1 && p <= 65535))
}

normalize_port() {
  local name="$1"
  local value="$2"
  local fallback="$3"
  if is_valid_port "$value"; then
    printf '%s' "$value"
  else
    log "invalid ${name}=${value}; using ${fallback}"
    printf '%s' "$fallback"
  fi
}

is_env_enabled() {
  case "$(printf '%s' "${1:-0}" | tr '[:upper:]' '[:lower:]')" in
    1|true|on|yes) return 0 ;;
    *) return 1 ;;
  esac
}

export DATA_DIR="${GRAIL_DATA_DIR:-/data}"
export DISPLAY="${GRAIL_BROWSER_DISPLAY:-:1}"

PROFILE_NAME="${GRAIL_BROWSER_PROFILE_NAME:-${OPENCLAW_BROWSER_PROFILE_NAME:-default}}"
if [ -z "${PROFILE_NAME}" ]; then
  PROFILE_NAME="default"
fi

BASE_HOME="${GRAIL_BROWSER_HOME:-${OPENCLAW_BROWSER_HOME:-${DATA_DIR}/browser-profiles}}"
export HOME="${BASE_HOME%/}/${PROFILE_NAME}"
export XDG_CONFIG_HOME="${HOME}/.config"
export XDG_CACHE_HOME="${HOME}/.cache"

CDP_PORT_RAW="${GRAIL_BROWSER_CDP_PORT:-${OPENCLAW_BROWSER_CDP_PORT:-9222}}"
VNC_PORT_RAW="${GRAIL_BROWSER_VNC_PORT:-${OPENCLAW_BROWSER_VNC_PORT:-5900}}"
NOVNC_PORT_RAW="${GRAIL_BROWSER_NOVNC_PORT:-${OPENCLAW_BROWSER_NOVNC_PORT:-6080}}"

CDP_PORT="$(normalize_port GRAIL_BROWSER_CDP_PORT "${CDP_PORT_RAW}" 9222)"
VNC_PORT="$(normalize_port GRAIL_BROWSER_VNC_PORT "${VNC_PORT_RAW}" 5900)"
NOVNC_PORT="$(normalize_port GRAIL_BROWSER_NOVNC_PORT "${NOVNC_PORT_RAW}" 6080)"

ENABLE_NOVNC="${GRAIL_BROWSER_ENABLE_NOVNC:-${OPENCLAW_BROWSER_ENABLE_NOVNC:-1}}"
HEADLESS="${GRAIL_BROWSER_HEADLESS:-${OPENCLAW_BROWSER_HEADLESS:-0}}"
SCREEN_SIZE="${GRAIL_BROWSER_SCREEN_SIZE:-1280x800x24}"
CDP_BIND="${GRAIL_BROWSER_CDP_BIND:-127.0.0.1}"
NOVNC_BIND="${GRAIL_BROWSER_NOVNC_BIND:-0.0.0.0}"
NOVNC_WEB_PATH="${GRAIL_BROWSER_NOVNC_WEB_PATH:-/usr/share/novnc/}"

if [[ "${CDP_BIND}" != "127.0.0.1" && "${CDP_BIND}" != "0.0.0.0" ]]; then
  log "invalid GRAIL_BROWSER_CDP_BIND=${CDP_BIND}; using 127.0.0.1"
  CDP_BIND="127.0.0.1"
fi

require_bin chromium
require_bin curl
require_bin Xvfb
require_bin socat

if is_env_enabled "${ENABLE_NOVNC}" && [[ "${HEADLESS}" != "1" ]]; then
  require_bin x11vnc
  require_bin websockify
fi

mkdir -p "${HOME}" "${HOME}/.chrome" "${XDG_CONFIG_HOME}" "${XDG_CACHE_HOME}"
chmod 700 "${HOME}" "${HOME}/.chrome" "${XDG_CONFIG_HOME}" "${XDG_CACHE_HOME}" || true

pids=()
cleanup() {
  trap - EXIT INT TERM
  for pid in "${pids[@]:-}"; do
    kill "${pid}" >/dev/null 2>&1 || true
  done
  wait >/dev/null 2>&1 || true
}

trap cleanup EXIT
trap 'cleanup; exit 143' INT TERM

start_bg() {
  "$@" &
  pids+=("$!")
}

start_bg Xvfb "${DISPLAY}" -screen 0 "${SCREEN_SIZE}" -ac -nolisten tcp

if [[ "${HEADLESS}" == "1" ]]; then
  CHROME_ARGS=(
    "--headless=new"
    "--disable-gpu"
  )
else
  CHROME_ARGS=()
fi

if ((CDP_PORT >= 65535)); then
  CHROME_CDP_PORT="$((CDP_PORT - 1))"
else
  CHROME_CDP_PORT="$((CDP_PORT + 1))"
fi

CHROME_ARGS+=(
  "--remote-debugging-address=127.0.0.1"
  "--remote-debugging-port=${CHROME_CDP_PORT}"
  "--user-data-dir=${HOME}/.chrome"
  "--no-first-run"
  "--no-default-browser-check"
  "--disable-dev-shm-usage"
  "--disable-background-networking"
  "--disable-features=TranslateUI"
  "--disable-breakpad"
  "--disable-crash-reporter"
  "--metrics-recording-only"
  "--password-store=basic"
  "--use-mock-keychain"
  "--no-sandbox"
)

if [ -n "${GRAIL_BROWSER_CHROME_ARGS:-}" ]; then
  # shellcheck disable=SC2206
  EXTRA_CHROME_ARGS=( ${GRAIL_BROWSER_CHROME_ARGS} )
  CHROME_ARGS+=("${EXTRA_CHROME_ARGS[@]}")
fi

start_bg chromium "${CHROME_ARGS[@]}" about:blank

cdp_ready=0
for _ in $(seq 1 100); do
  if curl -fsS --max-time 1 "http://127.0.0.1:${CHROME_CDP_PORT}/json/version" >/dev/null; then
    cdp_ready=1
    break
  fi
  sleep 0.1
done

if [[ "${cdp_ready}" != "1" ]]; then
  log "chromium CDP endpoint did not become ready on 127.0.0.1:${CHROME_CDP_PORT}"
  exit 1
fi

start_bg socat \
  "TCP-LISTEN:${CDP_PORT},fork,reuseaddr,bind=${CDP_BIND}" \
  "TCP:127.0.0.1:${CHROME_CDP_PORT}"

if is_env_enabled "${ENABLE_NOVNC}" && [[ "${HEADLESS}" != "1" ]]; then
  X11VNC_ARGS=(
    -display "${DISPLAY}"
    -rfbport "${VNC_PORT}"
    -shared
    -forever
    -localhost
  )

  if [ -n "${GRAIL_BROWSER_VNC_PASSWORD:-}" ]; then
    X11VNC_ARGS+=( -passwd "${GRAIL_BROWSER_VNC_PASSWORD}" )
  else
    X11VNC_ARGS+=( -nopw )
    log "warning: noVNC is enabled without GRAIL_BROWSER_VNC_PASSWORD"
  fi

  start_bg x11vnc "${X11VNC_ARGS[@]}"
  start_bg websockify --web "${NOVNC_WEB_PATH}" "${NOVNC_BIND}:${NOVNC_PORT}" "localhost:${VNC_PORT}"
fi

wait -n
