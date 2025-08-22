import React, { useState } from "react";
import Login, { Password, Username, Submit } from "@react-login-page/base";

const App = () => {
  const [status, setStatus] = useState("");

  const handle = (event) => {
    event.preventDefault();
    if (!event.currentTarget[0].value) {
      setStatus("Username cannot be empty");
      return;
    }
    if (event.currentTarget[0].value != "admin") {
      setStatus("Invalid username");
      return;
    }
    if (!event.currentTarget[1].value) {
      setStatus("Password cannot be empty");
      return;
    }
    if (event.currentTarget[1].value != "666666") {
      setStatus("Invalid password");
      return;
    }
    setStatus("Login success");
  };

  return (
    <div>
    <p style={{textAlign: 'center'}}>Auto testing started, press Command + Option + I to view the process.</p>
    <form method="post" onSubmit={handle}>
      <Login style={{ height: "100vh" }}>
        <Username name="username" />
        <Password name="password" />
        <Submit>submit</Submit>
        {status && <div>{status}</div>}
      </Login>
    </form>
    </div>
  );
};

export default App;
