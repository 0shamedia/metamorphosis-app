# script_check_insightface.py
import sys
try:
    import insightface
    print("SUCCESS: insightface imported")
    # Optionally print version: # print(f"Insightface version: {insightface.__version__}")
    sys.exit(0)
except ImportError:
    print("ERROR: Failed to import insightface")
    sys.exit(1)
except Exception as e:
    print(f"ERROR: Other error with insightface: {e}")
    sys.exit(1)