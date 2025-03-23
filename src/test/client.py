import requests
import json

default_request = """
{
    "updater":"hypertrail",
    "acceptformat":"json",
    "hw":{
            "sse":1,
            "sse2":1,
            "sse41":1,
            "sse42":1,
            "sse3":1,
            "avx":1,
            "physmemory":10
    },
    "ismachine":0,
    "os":{
            "platform":"Linux",
            "sp":"",
            "arch":"x86",
            "dedup":"cr"
    },
    "protocol":1.0,
    "requestid":"",
    "sessionid":"",
    "channel":"Dev",
    "updaterversion":0.1
}
"""

default_status_request = """
{
    "eventtype":"",
    "action":"",
    "result":1,
    "request": {
        "updater":"hypertrail",
        "acceptformat":"json",
        "hw":{
                "sse":1,
                "sse2":1,
                "sse41":1,
                "sse42":1,
                "sse3":1,
                "avx":1,
                "physmemory":10
        },
        "ismachine":0,
        "os":{
                "platform":"Linux",
                "sp":"",
                "arch":"x86",
                "dedup":"cr"
        },
        "protocol":1.0,
        "requestid":"",
        "sessionid":"",
        "channel":"Dev",
        "updaterversion":0.1
    }
}
"""

def latest():
    latest_request = requests.get('http://localhost:7778/latest', data=default_request)
    if(latest_request.status_code != 200):
        print("Failed:" + latest_request.status_code)
    return latest_request

def latest_download_session():
    latest_request = latest() 
    if latest_request.status_code == 200:
        latest_response = latest_request.json()
        sessionid = latest_response['sessionid']
        requestid = latest_response['requestid']
        request_obj = json.loads(default_request)
        request_obj['sessionid'] = sessionid
        request_obj['requestid'] = requestid

        download_request = requests.get('http://localhost:7778/download', data=json.dumps(request_obj))
        if download_request.status_code != 200:
            print("Failed:" + download_request.status_code)
        return download_request

def latest_download_status():
    download_request = latest_download_session()
    download_response = download_request.json()
    sessionid = download_response['sessionid']
    requestid = download_response['requestid']

    request_obj = json.loads(default_request)
    request_obj['sessionid'] = sessionid
    request_obj['requestid'] = requestid

    status_request_obj = json.loads(default_status_request)
    status_request_obj['request'] = request_obj
    status_request_obj['action'] = "retry"
    status_request_obj['eventtype'] = "Download"
    status_request_obj['result'] = 1

    status_request = requests.get('http://localhost:7778/status', data=json.dumps(status_request_obj))
    if status_request.status_code != 200 :
        print("Failed:" + status_request.status_code)
    return status_request

def main():
    print("main function")
    latest()
    latest_download_session()
    latest_download_status()

if __name__ == "__main__":
    main()
