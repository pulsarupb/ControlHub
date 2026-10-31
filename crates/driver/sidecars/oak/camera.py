"""DepthAI v3 capture adapted from OakD-Test; no web server or SDK import in tests."""
import time
from contextlib import suppress


def capture(mxid, config, fps, publish):
    import cv2
    import depthai as dai
    import numpy as np

    device = dai.Device(dai.DeviceInfo(mxid))
    pipeline = None
    try:
        pipeline = dai.Pipeline(defaultDevice=device)
        rgb = pipeline.create(dai.node.Camera).build(
            boardSocket=dai.CameraBoardSocket.CAM_A, sensorFps=fps)
        if config["rotate_180"]:
            rgb.setImageOrientation(dai.CameraImageOrientation.ROTATE_180_DEG)
        raw = rgb.requestOutput(size=(640, 400), type=dai.ImgFrame.Type.NV12, fps=fps)
        encoder = pipeline.create(dai.node.VideoEncoder)
        encoder.setDefaultProfilePreset(fps, dai.VideoEncoderProperties.Profile.MJPEG)
        encoder.setQuality(80)
        raw.link(encoder.input)

        left = pipeline.create(dai.node.Camera).build(
            boardSocket=dai.CameraBoardSocket.CAM_B, sensorFps=fps)
        right = pipeline.create(dai.node.Camera).build(
            boardSocket=dai.CameraBoardSocket.CAM_C, sensorFps=fps)
        stereo = pipeline.create(dai.node.StereoDepth)
        stereo.setDefaultProfilePreset(dai.node.StereoDepth.PresetMode.HIGH_DETAIL)
        stereo.setDepthAlign(dai.CameraBoardSocket.CAM_A)
        stereo.setOutputSize(640, 400)
        stereo.setLeftRightCheck(True)
        stereo.setSubpixel(False)
        cfg = stereo.initialConfig
        cfg.costMatching.confidenceThreshold = 55
        cfg.postProcessing.speckleFilter.enable = True
        cfg.postProcessing.speckleFilter.speckleRange = 200
        cfg.postProcessing.spatialFilter.enable = True
        cfg.postProcessing.spatialFilter.holeFillingRadius = 2
        cfg.postProcessing.spatialFilter.numIterations = 1
        cfg.postProcessing.thresholdFilter.minRange = 300
        cfg.postProcessing.thresholdFilter.maxRange = 20000
        left.requestOutput(size=(640, 400), type=dai.ImgFrame.Type.GRAY8, fps=fps).link(stereo.left)
        right.requestOutput(size=(640, 400), type=dai.ImgFrame.Type.GRAY8, fps=fps).link(stereo.right)

        metrics = pipeline.create(dai.node.SpatialLocationCalculator)
        roi = dai.SpatialLocationCalculatorConfigData()
        roi.roi = dai.Rect(dai.Point2f(0, 0), dai.Point2f(1, 1))
        roi.calculationAlgorithm = dai.SpatialLocationCalculatorAlgorithm.MEAN
        metrics.initialConfig.addROI(roi)
        stereo.depth.link(metrics.inputDepth)
        q_rgb = encoder.bitstream.createOutputQueue(maxSize=1, blocking=False)
        q_depth = stereo.disparity.createOutputQueue(maxSize=1, blocking=False)
        q_meta = metrics.out.createOutputQueue(maxSize=1, blocking=False)
        pipeline.start()
        last_frame = time.monotonic()
        while pipeline.isRunning():
            got_frame = False
            frame = q_rgb.tryGet()
            if frame is not None:
                publish("color", bytes(frame.getData()))
                got_frame = True
            frame = q_depth.tryGet()
            if frame is not None:
                disparity = frame.getCvFrame()
                valid = disparity > 0
                normalized = np.zeros(disparity.shape, dtype=np.uint8)
                if valid.any():
                    lo, hi = disparity[valid].min(), disparity[valid].max()
                    normalized[valid] = ((disparity[valid].astype(np.float32) - lo) * 255 / (hi - lo)).astype(np.uint8) if lo < hi else 128
                colored = cv2.applyColorMap(normalized, cv2.COLORMAP_JET)
                colored[~valid] = (255, 0, 255)
                if config["rotate_180"]:
                    colored = cv2.rotate(colored, cv2.ROTATE_180)
                ok, jpeg = cv2.imencode(".jpg", colored, [cv2.IMWRITE_JPEG_QUALITY, 80])
                if ok:
                    publish("depth", jpeg.tobytes())
                    got_frame = True
            frame = q_meta.tryGet()
            if frame is not None:
                locations = frame.getSpatialLocations()
                if locations:
                    sd = locations[0]
                    publish("metrics", {"min_mm": int(sd.depthMin), "avg_mm": int(sd.depthAverage), "max_mm": int(sd.depthMax)})
            if got_frame:
                last_frame = time.monotonic()
            if time.monotonic() - last_frame > 5:
                raise RuntimeError("Camera capture stalled; reconnecting")
            time.sleep(0.002)
        raise RuntimeError("Camera disconnected; reconnecting")
    finally:
        if pipeline is not None:
            with suppress(Exception):
                pipeline.stop()
            with suppress(Exception):
                pipeline.wait()
        with suppress(Exception):
            device.close()
