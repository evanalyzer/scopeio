#!/bin/sh
# Copyright 2026 Joachim Danmayr.
# Licensed under the AGPL-3.0-only with the EVAnalyzer exception.
# See LICENSE and LICENSE-EXCEPTION.
#
# Regenerates the Bruker Prairie View references: CatalystNeuro test
# recordings (GIN ophys_testing_data, fetched with tests/scripts/fetch_gin.py)
# and the synthetic recordings.
#
# Usage: REP=3 sh tests/scripts/oracle_prairie.sh
set -e
cd "$(dirname "$0")/../.."
B=tests/reference/gin-ophys/imaging_datasets/BrukerTif
REP=${REP:-1}
python3 -I tests/scripts/oracle.py prairie --repeat $REP $(ls $B/*/*.xml | grep -v Cycle0) tests/data/prairie/*/*.xml
