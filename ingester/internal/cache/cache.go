package cache

import (
	"context"
	"sync"
	"time"
)

// Cache is a generic thread-safe store with optional TTL.
type Cache[K comparable, V any] struct {
	mu    sync.RWMutex
	items map[K]*CacheEntry[V]
	ttl   time.Duration
}

type CacheEntry[V any] struct {
	Value     V
	Found     bool
	ExpiresAt time.Time
}

// NewCache creates a cache. ttl=0 means entries never expire.
func NewCache[K comparable, V any](ttl time.Duration) *Cache[K, V] {
	return &Cache[K, V]{
		items: make(map[K]*CacheEntry[V]),
		ttl:   ttl,
	}
}

// Get returns (value, found, exists). Expired entries are evicted on read.
func (c *Cache[K, V]) Get(key K) (V, bool, bool) {
	c.mu.RLock()
	entry, exists := c.items[key]

	if !exists {
		c.mu.RUnlock()
		var zero V
		return zero, false, false
	}

	if !entry.ExpiresAt.IsZero() && time.Now().After(entry.ExpiresAt) {
		c.mu.RUnlock()
		// Upgrade to write lock to evict
		c.mu.Lock()
		delete(c.items, key)
		c.mu.Unlock()
		var zero V
		return zero, false, false
	}

	c.mu.RUnlock()
	return entry.Value, entry.Found, true
}

// Set stores a value. found=false caches a negative lookup.
func (c *Cache[K, V]) Set(key K, value V, found bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	var expiresAt time.Time
	if c.ttl > 0 {
		expiresAt = time.Now().Add(c.ttl)
	}

	c.items[key] = &CacheEntry[V]{
		Value:     value,
		Found:     found,
		ExpiresAt: expiresAt,
	}
}

// GetOrFetch returns a cached value or calls fetch on miss.
// Failures are only cached when TTL > 0 so transient errors don't become permanent.
func (c *Cache[K, V]) GetOrFetch(
	ctx context.Context,
	key K,
	fetch func(context.Context, K) (V, bool),
) (V, bool) {
	value, found, exists := c.Get(key)
	if exists {
		return value, found
	}

	value, found = fetch(ctx, key)

	if found || c.ttl > 0 {
		c.Set(key, value, found)
	}

	return value, found
}

func (c *Cache[K, V]) Size() int {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return len(c.items)
}

func (c *Cache[K, V]) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.items = make(map[K]*CacheEntry[V])
}

func (c *Cache[K, V]) Evict(key K) {
	c.mu.Lock()
	defer c.mu.Unlock()
	delete(c.items, key)
}

func (c *Cache[K, V]) EvictExpired() int {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.ttl == 0 {
		return 0
	}

	now := time.Now()
	evicted := 0

	for key, entry := range c.items {
		if !entry.ExpiresAt.IsZero() && now.After(entry.ExpiresAt) {
			delete(c.items, key)
			evicted++
		}
	}

	return evicted
}
