package storage

type Reader interface {
    LatestTWAP(pair string) (float64, bool)
    LatestVolume(pair string) (float64, bool)
    Summary() map[string]any
}

type Writer interface {
    Upsert(pair string, twap, vol float64)
}
