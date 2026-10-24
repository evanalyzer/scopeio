#!/bin/sh
# Regenerates the Imaris IMS references (OME sample server, CC BY 4.0).
#
# Usage: REP=3 sh tests/scripts/oracle_ims.sh
set -e
cd "$(dirname "$0")/../.."
I=tests/reference/ome/Imaris-IMS
REP=${REP:-1}
python3 -I tests/scripts/oracle.py ims --repeat $REP --per-series $I/davemason/Convallaria_3C_1T_confocal.ims $I/davemason/Convallaria_3C_1T_2x2grid_confocal.ims $I/eliana/retina_large.ims "$I/eliana/2016-07-26_17.11.03_Sample Prefix_5P298C2_Protocol.ims"
