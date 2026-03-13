package api

import (
	"encoding/json"
	"io"
	"net/http"

	"api/internal/storage"

	"github.com/gin-gonic/gin"
)

func SetupRoutes(r *gin.Engine, store storage.Reader) {
	r.GET("/health", func(c *gin.Context) { c.Status(http.StatusOK) })

	// DAPR subscription endpoint for analytics events
	r.POST("/events/analytics", func(c *gin.Context) {
		body, err := io.ReadAll(c.Request.Body)
		if err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "failed to read body"})
			return
		}

		// Parse CloudEvents envelope from DAPR
		var cloudEvent struct {
			Data json.RawMessage `json:"data"`
		}
		if err := json.Unmarshal(body, &cloudEvent); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid cloudevents format"})
			return
		}

		// Parse analytics payload
		var payload struct {
			PairAddress string  `json:"pairAddress"`
			TWAP        float64 `json:"twap"`
			VolumeUSD   float64 `json:"volumeUSD"`
		}
		if err := json.Unmarshal(cloudEvent.Data, &payload); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": "invalid analytics payload"})
			return
		}

		// Update storage
		if writer, ok := store.(storage.Writer); ok {
			writer.Upsert(payload.PairAddress, payload.TWAP, payload.VolumeUSD)
		}

		c.Status(http.StatusOK)
	})

	r.GET("/pairs/:pair/twap", func(c *gin.Context) {
		pair := c.Param("pair")
		twap, ok := store.LatestTWAP(pair)
		if !ok {
			c.JSON(http.StatusNotFound, gin.H{"error": "pair not found"})
			return
		}
		c.JSON(http.StatusOK, gin.H{"pair": pair, "twap": twap})
	})

	r.GET("/pairs/:pair/volume", func(c *gin.Context) {
		pair := c.Param("pair")
		vol, ok := store.LatestVolume(pair)
		if !ok {
			c.JSON(http.StatusNotFound, gin.H{"error": "pair not found"})
			return
		}
		c.JSON(http.StatusOK, gin.H{"pair": pair, "volume": vol})
	})

	r.GET("/analytics/summary", func(c *gin.Context) {
		c.JSON(http.StatusOK, store.Summary())
	})
}
