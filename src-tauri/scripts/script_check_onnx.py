# script_check_onnx.py
import sys
import os
import traceback

print(f"DEBUG: sys.path: {sys.path}")
print(f"DEBUG: PATH environment variable: {os.environ.get('PATH')}")

try:
    import onnxruntime
    print("SUCCESS: onnxruntime imported")
    print(f"ONNX Runtime version: {onnxruntime.__version__}")
    print(f"ONNX Runtime providers: {onnxruntime.get_available_providers()}")
    sys.exit(0)
except ImportError as e:
    print(f"ERROR: Failed to import onnxruntime: {e}")
    traceback.print_exc()
    sys.exit(1)
except Exception as e:
    print(f"ERROR: Other error with onnxruntime: {e}")
    traceback.print_exc()
    sys.exit(1)