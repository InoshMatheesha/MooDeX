import json
from rawg_api import RAWGApiClient

api = RAWGApiClient()
upcoming = api.get_upcoming_games()
trending = api.get_trending_games()
popular = api.get_popular_games()

print("Upcoming count:", len(upcoming))
print("Trending count:", len(trending))
print("Popular count:", len(popular))

with open("test_api_res.json", "w") as f:
    json.dump({"upcoming": upcoming, "trending": trending, "popular": popular}, f, indent=2)
