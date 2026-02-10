package api

import (
	"net/http"

	"api/internal/storage"

	"github.com/gin-gonic/gin"
)

func SetupRoutes(r *gin.Engine, store storage.Reader) {
	r.GET("/health", func(c *gin.Context) { c.Status(http.StatusOK) })

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
