#!/bin/bash
set -euo pipefail
if [ "$#" -lt 2 ]; then
  echo 'Usage: profile-scroll.sh SIMULATOR_OR_DEVICE_ID SESSION_ID [RESULT_BUNDLE]'
  exit 2
fi
cd "$(dirname "$0")/../../.."
result=${3:-/tmp/crabigator-scroll-$(date +%Y%m%d-%H%M%S).xcresult}
TEST_RUNNER_CRABIGATOR_TEST_SESSION_ID="$2" TEST_RUNNER_CRABIGATOR_PROFILE_SCROLL=1 xcodebuild \
  -project clients/ios/Crabigator.xcodeproj -scheme Crabigator \
  -destination "id=$1" -configuration Release -derivedDataPath clients/ios/build-profile \
  -only-testing:CrabigatorUITests/ScrollPerformanceTests \
  -resultBundlePath "$result" ENABLE_TESTABILITY=YES test
printf 'Complete gesture measurements: %s\n' "$result"
