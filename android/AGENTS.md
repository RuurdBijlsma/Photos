Check for performance regressions in timeline:

```powershell
./gradlew :benchmark:connectedBenchmarkAndroidTest

# When updating baseline:
node script/check-perf-regression.mjs --update-baseline

# Otherwise
node script/check-perf-regression.mjs
```
