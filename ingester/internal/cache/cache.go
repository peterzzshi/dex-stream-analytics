package cache

import (
	"context"
	"sync"
	"time"
)

// Cache provides a generic thread-safe caching mechanism with optional TTL
// K: key type (must be comparable), V: value type
type Cache[K comparable, V any] struct {
	mu    sync.RWMutex
	items map[K]*CacheEntry[V]
	ttl   time.Duration // 0 means no expiration
}

// CacheEntry stores a cached value with metadata
type CacheEntry[V any] struct {
	Value     V
	Found     bool      // false means "cache this failure"
	ExpiresAt time.Time // zero time means never expires
}

// NewCache creates a new cache with optional TTL
// ttl = 0 means items never expire (appropriate for immutable data like token symbols)
func NewCache[K comparable, V any](ttl time.Duration) *Cache[K, V] {
	return &Cache[K, V]{
		items: make(map[K]*CacheEntry[V]),
		ttl:   ttl,
	}
}

// Get retrieves a value from cache
// Returns (value, found, exists)
// - found: whether the lookup was successful (true) or cached failure (false)
// - exists: whether the key exists in cache at all
// Expired entries are automatically evicted to prevent memory leaks
func (c *Cache[K, V]) Get(key K) (V, bool, bool) {
	c.mu.RLock()
	entry, exists := c.items[key]

	if !exists {
		c.mu.RUnlock()
		var zero V
		return zero, false, false
	}

	// Check expiration
	if !entry.ExpiresAt.IsZero() && time.Now().After(entry.ExpiresAt) {
		c.mu.RUnlock()

		// Upgrade to write lock to evict expired entry
		c.mu.Lock()
		delete(c.items, key)
		c.mu.Unlock()

		var zero V
		return zero, false, false
	}

	c.mu.RUnlock()
	return entry.Value, entry.Found, true
}

// Set stores a value in cache
// found = true means successful lookup, false means cache a failure
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

// GetOrFetch retrieves from cache or fetches using provided function
// The fetch function is only called on cache miss
// Success is always cached. Failures are only cached if TTL > 0 (to allow retries).
func (c *Cache[K, V]) GetOrFetch(
	ctx context.Context,
	key K,
	fetch func(context.Context, K) (V, bool),
) (V, bool) {
	// Try cache first
	value, found, exists := c.Get(key)
	if exists {
		return value, found
	}

	// Cache miss - fetch from source
	value, found = fetch(ctx, key)

	// Cache the result - but don't cache failures when TTL is infinite
	// Rationale: transient RPC errors shouldn't be permanent with TTL=0
	if found || c.ttl > 0 {
		c.Set(key, value, found)
	}

	return value, found
}

// Size returns the number of items in cache
func (c *Cache[K, V]) Size() int {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return len(c.items)
}

// Clear removes all items from cache
func (c *Cache[K, V]) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.items = make(map[K]*CacheEntry[V])
}

// Evict removes a specific key from cache
func (c *Cache[K, V]) Evict(key K) {
	c.mu.Lock()
	defer c.mu.Unlock()
	delete(c.items, key)
}

// EvictExpired removes all expired entries (useful for periodic cleanup)
// Returns number of entries evicted
func (c *Cache[K, V]) EvictExpired() int {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.ttl == 0 {
		return 0 // No expiration configured
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
