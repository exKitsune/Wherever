package com.fruit.wherever;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Bundle;
import android.text.method.ScrollingMovementMethod;
import android.util.Log;
import android.view.View;
import android.widget.CompoundButton;
import android.widget.TextView;
import android.widget.ToggleButton;

import androidx.appcompat.app.ActionBar;
import androidx.appcompat.app.AppCompatActivity;
import androidx.preference.PreferenceFragmentCompat;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;

public class SettingsActivity extends AppCompatActivity {

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Intent intent = getIntent();
        SharedPreferences prefs = getPreferences(Context.MODE_PRIVATE);
        if(intent.getAction() == Intent.ACTION_SEND || intent.getAction() == Intent.ACTION_VIEW) {
            Uri uri = intent.getData();
            if(uri.getScheme() == "where") {
                String home_ip = uri.getHost();
                int home_port = uri.getPort();

                SharedPreferences.Editor editor = prefs.edit();
                editor.putString("ip", home_ip);
                editor.putInt("port", home_port);
                editor.apply();
            } else { //http https
                if (prefs.getBoolean("enabled", false)) {
                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    if (home_ip == "") {
                        return;
                    }
                    Runnable r = new Runnable() {
                        @Override
                        public void run() {
                            try {
                                URL url = new URL("http://" + home_ip + ":" + home_port + "/open");
                                HttpURLConnection con = (HttpURLConnection) url.openConnection();
                                con.setDoOutput(true);
                                con.setRequestMethod("POST");
                                con.setRequestProperty("Content-Type", "text/plain; utf-8");
                                try (OutputStream os = con.getOutputStream()) {
                                    byte[] input = uri.toString().getBytes("utf-8");
                                    os.write(input, 0, input.length);
                                }
                                con.getResponseCode();
                            } catch (Exception e) {
                                Log.e("BRUH", e.toString());
                            }
                        }
                    };
                    new Thread(r).start();
                } else {
                    Log.d("bruh", "intent chooser");
                    Log.d("bruh", intent.toString());
                    Intent i2 = Intent.createChooser(intent, "Open Link");
                    Log.d("bruh", i2.toString());
                    startActivity(i2);
                }
                finish();
                super.onBackPressed();
                return;
            }
        } else {
            setContentView(R.layout.settings_activity);
            ActionBar actionBar = getSupportActionBar();
            if (actionBar != null) {
                actionBar.setDisplayHomeAsUpEnabled(true);
            }

            boolean enabled = prefs.getBoolean("enabled", false);

            ToggleButton toggle = (ToggleButton) findViewById(R.id.on_off_button);
            toggle.setChecked(enabled);
            toggle.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
                public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {

                    String home_ip = prefs.getString("ip", "192.168.1.11");
                    int home_port = prefs.getInt("port", 8998);

                    SharedPreferences.Editor editor = prefs.edit();
                    if (isChecked) {
                        editor.putBoolean("enabled", true);
                    } else {
                        editor.putBoolean("enabled", false);
                    }

                    editor.apply();
                }
            });
        }
    }
}

