import type { PageServerLoadEvent } from "./$types";
import { redirect } from '@sveltejs/kit';

export async function load(event: PageServerLoadEvent) {

	// Clear the session cookie by setting it to an expired date
	event.cookies.delete('session', {
		path: '/',
	});

	// Redirect to the login page after logout
	return redirect(303, '/login');
}
