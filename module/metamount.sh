# Copyright (C) 2026 YuzakiKokuban <heibanbaize@gmail.com>
#
# This program is free software; you can redistribute it and/or
# modify it under the terms of the GNU General Public License
# as published by the Free Software Foundation; either version 2
# of the License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.

MODDIR="${0%/*}"
BASE_DIR="/data/adb/hybrid-mount"

mkdir -p "$BASE_DIR"

BINARY="$MODDIR/hybrid-mount"
if [ ! -f "$BINARY" ]; then
  echo "ERROR: Binary not found at $BINARY"
  exit 1
fi

chmod 755 "$BINARY"
"$BINARY" 2>&1
EXIT_CODE=$?

if [ "$EXIT_CODE" = "0" ] && [ -x /data/adb/ksud ]; then
  /data/adb/ksud kernel notify-module-mounted
fi
exit $EXIT_CODE
