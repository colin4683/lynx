import type {Actions, RequestEvent} from "./$types";
import { fail, redirect } from '@sveltejs/kit';
import { decodeBase64, encodeBase64 } from '@oslojs/encoding';
import { createTOTPKeyURI, verifyTOTP } from "@oslojs/otp";
import { renderSVG } from "uqr";
import { updateUserTOTPKey } from '$lib/server/user';
import { setSession2fa } from '$lib/server/session';

export async function load(event: RequestEvent) {
	if (event.locals.session === null || event.locals.user === null) {
		return redirect(302, "/login");
	}
	if (!event.locals.user.emailVerified) {
		return redirect(302, "/verify-email");
	}

	const totpKey = new Uint8Array(20);
	crypto.getRandomValues(totpKey);
	const encodedTOTPKey = encodeBase64(totpKey);
	const keyURI = createTOTPKeyURI("lynx-hub", event.locals.user.email, totpKey, 30, 6);
	const qrcode = renderSVG(keyURI);
	return {
		encodedTOTPKey,
		qrcode
	};
}


export const actions: Actions = {
	default: action
};

async function action(event: RequestEvent) {
	if (event.locals.session === null || event.locals.user === null) {
		return redirect(302, "/login");
	}
	if (!event.locals.user.emailVerified) {
		return redirect(302, "/verify-email");
	}


	const formData = await event.request.formData();
	const encodedKey = formData.get("key");
	const code = formData.get("code");
	if (typeof encodedKey !== "string" || typeof code !== "string") {
		return fail(400, {
			message: "Invalid or missing fields"
		});
	}
	if (code === "") {
		return fail(400, {
			message: "Please enter your code"
		});
	}
	if (encodedKey.length !== 28) {
		return fail(400, {
			message: "Please enter your code"
		});
	}
	let key: Uint8Array;
	try {
		key = decodeBase64(encodedKey);
	} catch {
		return fail(400, {
			message: "Invalid key"
		});
	}
	if (key.byteLength !== 20) {
		return fail(400, {
			message: "Invalid key"
		});
	}

	if (!verifyTOTP(key, 30, 6, code)) {
		return fail(400, {
			message: "Invalid code"
		})
	}

	await updateUserTOTPKey(event.locals.session.userId, key);
	await setSession2fa(event.locals.session.id);
	return redirect(302, "/");
}