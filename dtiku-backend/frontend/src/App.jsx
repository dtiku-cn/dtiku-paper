import * as React from "react";
import { Admin, Resource } from 'react-admin';
import restProvider from 'ra-data-simple-rest';
import TaskList from "./task/TaskList"

function App() {
  return (
    <Admin dataProvider={restProvider('http://localhost:3000')}>
      <Resource name="tasks" list={TaskList} />
    </Admin>
  );
}

export default App;
