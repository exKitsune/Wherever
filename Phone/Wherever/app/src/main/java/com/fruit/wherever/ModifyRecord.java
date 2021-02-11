package com.fruit.wherever;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.TextView;

public class ModifyRecord extends Activity implements View.OnClickListener {

    TextView modTextView;
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setTitle("Delete Record?");

        setContentView(R.layout.activity_modify_record);

        Button deleteBtn = (Button) findViewById(R.id.delete_btn);
        Button cancelBtn = (Button) findViewById(R.id.cancel_btn);
        modTextView = (TextView) findViewById(R.id.modTextView);
        Intent intent = getIntent();
        String host = intent.getStringExtra("host");
        modTextView.setText(host);

        deleteBtn.setOnClickListener(this);
        cancelBtn.setOnClickListener(this);
    }

    @Override
    public void onClick(View v) {
        switch(v.getId()) {
            case R.id.cancel_btn:
                this.returnHome();
                break;
            case R.id.delete_btn:
                String host = modTextView.getText().toString();
                DBManager.getInstance(getApplicationContext()).delete(host);
                this.returnHome();
                break;
        }
    }

    public void returnHome() {
        Intent home_intent = new Intent(getApplicationContext(), SettingsActivity.class)
                .setFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP);
        startActivity(home_intent);
    }
}
