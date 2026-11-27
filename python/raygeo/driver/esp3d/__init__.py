"""ESP3D / FluidNC connection parameters.

The driver is the GRBL network driver: commands travel over an HTTP
endpoint and a WebSocket response channel; jobs are uploaded and
executed instead of being streamed line by line.

The configuration class lives in :mod:`raygeo.driver.config`.
"""
