# Politehnica offline map

`politehnica.json` contains OpenStreetMap vector data for Bucharest around
Politehnica University (center 44.4385 N, 26.048 E). Coverage is approximately
10 × 10 km: south 44.3935, west 25.985, north 44.4835, east 26.111.

Data © [OpenStreetMap contributors](https://www.openstreetmap.org/copyright),
available under the [Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
This extract is distributed under ODbL 1.0. The widget displays attribution.
The JSON contains the extraction date, bounds, center, and vector features.
Coordinates are Web Mercator meters relative to the center, rounded to 0.1 m;
this rounding is not a statement of map accuracy.

To refresh, from `app/` run `bun scripts/build-rtk-map.ts` with internet access.
The script queries Overpass for vector data, not public raster tile servers.
Normal application use and builds do not contact Overpass or any tile service.
The JSON is shipped as a static asset with web and desktop builds.

Add **RTK Map (Offline)** from the dashboard widget picker. Pan by dragging or
using arrow keys, zoom with the buttons or mouse wheel. **Campus** returns to
the bundled area; **Recenter** enables following the current fix.
The in-memory trail keeps the last 2,000 fresh fixes and resets across gaps
longer than three seconds. Map coordinates are approximate: this is an
orientation display, not a surveyed map or autonomous navigation source.
An offline basemap does not provide offline NTRIP corrections.
