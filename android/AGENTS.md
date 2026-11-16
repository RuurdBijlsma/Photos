Check for performance regressions in timeline:

Ensure `dev.ruurd.photos` is signed in on the test device, media permissions are granted, and the
timeline is non-empty.

```powershell
./gradlew :benchmark:connectedBenchmarkAndroidTest

# When updating baseline:
./gradlew :benchmark:updatePerfBaseline

# Otherwise (check latest benchmark output against perf/baseline.json):
./gradlew :benchmark:checkPerfRegression

# Or run benchmarks and check in one command:
./gradlew :benchmark:benchmarkAndCheck
```
