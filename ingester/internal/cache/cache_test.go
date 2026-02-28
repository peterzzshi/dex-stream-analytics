package cache

import (
	"context"
	"testing"
	"time"
)

func TestCacheBasicOperations(t *testing.T) {
	cache := NewCache[string, int](0) // No expiration

	// Test Set and Get
	cache.Set("key1", 100, true)
	value, found, exists := cache.Get("key1")

	if !exists {
		t.Error("Expected key to exist")
	}
	if !found {
		t.Error("Expected found to be true")
	}
	if value != 100 {
		t.Errorf("Expected 100, got %d", value)
	}

	// Test non-existent key
	_, _, exists = cache.Get("key2")
	if exists {
		t.Error("Expected key to not exist")
	}
}

func TestCacheFailures(t *testing.T) {
	cache := NewCache[string, string](0)

	// Cache a failure (found = false)
	cache.Set("badkey", "", false)

	value, found, exists := cache.Get("badkey")
	if !exists {
		t.Error("Expected key to exist in cache")
	}
	if found {
		t.Error("Expected found to be false (cached failure)")
	}
	if value != "" {
		t.Errorf("Expected empty string, got %s", value)
	}
}

func TestCacheGetOrFetch(t *testing.T) {
	cache := NewCache[int, string](0)
	fetchCount := 0

	fetch := func(ctx context.Context, key int) (string, bool) {
		fetchCount++
		if key == 1 {
			return "one", true
		}
		return "", false
	}

	// First call - should fetch
	value, found := cache.GetOrFetch(context.Background(), 1, fetch)
	if !found || value != "one" {
		t.Errorf("Expected (one, true), got (%s, %v)", value, found)
	}
	if fetchCount != 1 {
		t.Errorf("Expected 1 fetch, got %d", fetchCount)
	}

	// Second call - should use cache
	value, found = cache.GetOrFetch(context.Background(), 1, fetch)
	if !found || value != "one" {
		t.Errorf("Expected (one, true), got (%s, %v)", value, found)
	}
	if fetchCount != 1 {
		t.Errorf("Expected still 1 fetch (cached), got %d", fetchCount)
	}

	// Fetch failure - should also be cached
	value, found = cache.GetOrFetch(context.Background(), 2, fetch)
	if found || value != "" {
		t.Errorf("Expected ('', false), got (%s, %v)", value, found)
	}
	if fetchCount != 2 {
		t.Errorf("Expected 2 fetches, got %d", fetchCount)
	}

	// Second call to failed key - should use cached failure
	value, found = cache.GetOrFetch(context.Background(), 2, fetch)
	if found || value != "" {
		t.Errorf("Expected ('', false), got (%s, %v)", value, found)
	}
	if fetchCount != 2 {
		t.Errorf("Expected still 2 fetches (cached), got %d", fetchCount)
	}
}

func TestCacheTTL(t *testing.T) {
	cache := NewCache[string, int](100 * time.Millisecond)

	// Set a value
	cache.Set("key1", 42, true)

	// Should exist immediately
	value, found, exists := cache.Get("key1")
	if !exists || !found || value != 42 {
		t.Error("Expected value to exist")
	}

	// Wait for expiration
	time.Sleep(150 * time.Millisecond)

	// Should be expired
	_, _, exists = cache.Get("key1")
	if exists {
		t.Error("Expected value to be expired")
	}
}

func TestCacheEvictExpired(t *testing.T) {
	cache := NewCache[string, int](50 * time.Millisecond)

	// Add multiple entries
	cache.Set("key1", 1, true)
	cache.Set("key2", 2, true)
	cache.Set("key3", 3, true)

	if cache.Size() != 3 {
		t.Errorf("Expected size 3, got %d", cache.Size())
	}

	// Wait for expiration
	time.Sleep(100 * time.Millisecond)

	// Evict expired entries
	evicted := cache.EvictExpired()
	if evicted != 3 {
		t.Errorf("Expected 3 evictions, got %d", evicted)
	}

	if cache.Size() != 0 {
		t.Errorf("Expected size 0 after eviction, got %d", cache.Size())
	}
}

func TestCacheClear(t *testing.T) {
	cache := NewCache[string, int](0)

	cache.Set("key1", 1, true)
	cache.Set("key2", 2, true)

	if cache.Size() != 2 {
		t.Errorf("Expected size 2, got %d", cache.Size())
	}

	cache.Clear()

	if cache.Size() != 0 {
		t.Errorf("Expected size 0 after clear, got %d", cache.Size())
	}
}

func TestCacheEvictSingle(t *testing.T) {
	cache := NewCache[string, int](0)

	cache.Set("key1", 1, true)
	cache.Set("key2", 2, true)

	cache.Evict("key1")

	_, _, exists := cache.Get("key1")
	if exists {
		t.Error("Expected key1 to be evicted")
	}

	_, _, exists = cache.Get("key2")
	if !exists {
		t.Error("Expected key2 to still exist")
	}
}

func TestCacheConcurrency(t *testing.T) {
	cache := NewCache[int, int](0)

	// Concurrent writes and reads
	done := make(chan bool)
	for i := 0; i < 10; i++ {
		go func(n int) {
			for j := 0; j < 100; j++ {
				cache.Set(n, n*100+j, true)
				cache.Get(n)
			}
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}

	// Should have 10 entries
	if cache.Size() != 10 {
		t.Errorf("Expected size 10, got %d", cache.Size())
	}
}
