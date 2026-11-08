package com.vandam.ink

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import org.json.JSONObject
import org.json.JSONArray

class ContractInstrumentation : Instrumentation() {
    override fun onCreate(arguments: Bundle?) {
        super.onCreate(arguments)
        start()
    }

    override fun onStart() {
        var passed = 0
        val failures = mutableListOf<String>()
        val test: (String, () -> Unit) -> Unit = { name, test ->
            try {
                test()
                passed++
                sendStatus(0, Bundle().apply { putString("stream", "PASS $name\n") })
            } catch (error: Throwable) {
                val detail = "$name: ${error.stackTraceToString()}"
                failures.add(detail)
                sendStatus(0, Bundle().apply { putString("stream", "FAIL $detail\n") })
            }
        }
        PermissionContracts(this).run(test)
        ContractTests(targetContext).run(test)
        finish(Activity.RESULT_OK, Bundle().apply {
            putString("ink.summary", JSONObject().put("passed", passed).put("failures", JSONArray(failures)).toString())
        })
    }
}
