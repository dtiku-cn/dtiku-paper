import * as React from "react";
import { Admin, Resource } from "react-admin";
import TaskList from "./task/TaskList";
import ConfigList from "./config/ConfigList";
import restProvider from "./rest";

function App() {
  return (
    <Admin dataProvider={restProvider("http://localhost:3000/api")}>
      <Resource name="tasks" list={TaskList} />
      <Resource name="configs" list={ConfigList} />
    </Admin>
  );
}

export default App;
