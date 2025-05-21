# script_check_onnx.py
import sys
try:
    import onnxruntime
    print("SUCCESS: onnxruntime imported")
    # Optionally print version: print(f"ONNX Runtime version: {onnxruntime.__version__}")
    # Optionally print providers: print(f"ONNX Runtime providers: {onnxruntime.get_available_providers()}")
    sys.exit(0)
except ImportError:
    print("ERROR: Failed to import onnxruntime")
    sys.exit(1)
except Exception as e:
    print(f"ERROR: Other error with onnxruntime: {e}")
    sys.exit(1)